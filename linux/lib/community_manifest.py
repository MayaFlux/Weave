import json
import math
import re
from urllib.parse import urlsplit


class ManifestError(ValueError):
    def __init__(self, path, reason):
        self.path = path
        self.reason = reason
        super().__init__(f"{path}: {reason}")


class _ObjectPairs(list):
    pass


class _NonIntegerNumber(str):
    pass


def parse_manifest(source, expected_name=None):
    if not isinstance(source, str):
        raise ManifestError("$", "Expected UTF-8 JSON text")

    try:
        decoded = json.loads(
            source.lstrip("\ufeff"),
            object_pairs_hook=_ObjectPairs,
            parse_float=_NonIntegerNumber,
            parse_constant=_NonIntegerNumber,
        )
    except (ValueError, RecursionError) as error:
        raise ManifestError("$", "Invalid JSON") from error

    def materialize(value, path, depth=0):
        if isinstance(value, _NonIntegerNumber):
            raise ManifestError(path, "Expected integer JSON number")

        if isinstance(value, list) and depth >= 64:
            raise ManifestError("$", "JSON nesting exceeds supported depth")

        if isinstance(value, _ObjectPairs):
            result = {}

            for key, child in value:
                if key in result:
                    raise ManifestError(path, f"Duplicate object key: {key}")

                result[key] = materialize(child, f"{path}.{key}", depth + 1)

            return result

        if isinstance(value, list):
            return [
                materialize(child, f"{path}[{index}]", depth + 1)
                for index, child in enumerate(value)
            ]
        if isinstance(value, float) and not math.isfinite(value):
            raise ManifestError(path, "Nonstandard or nonfinite number")

        return value

    try:
        manifest = materialize(decoded, "$")
        _validate_manifest(manifest, expected_name)
        return manifest
    except RecursionError as error:
        raise ManifestError("$", "JSON nesting exceeds supported depth") from error


def _object(value, path, allowed, required=()):
    if not isinstance(value, dict):
        raise ManifestError(path, "Expected object")

    for key in value:
        if key not in allowed:
            raise ManifestError(f"{path}.{key}", "Unknown field")

    for key in required:
        if key not in value:
            raise ManifestError(f"{path}.{key}", "Required field is missing")


def _text(value, path, nonempty=False):
    if not isinstance(value, str) or (nonempty and not value):
        raise ManifestError(
            path, "Expected nonempty string" if nonempty else "Expected string"
        )

    if re.search(r"[\x00-\x1f\x7f]", value):
        raise ManifestError(path, "Control characters are not allowed")


def _pattern(value, path, pattern):
    _text(value, path, True)

    if re.fullmatch(pattern, value) is None:
        raise ManifestError(path, "Invalid identifier")


def _boolean(value, path):
    if not isinstance(value, bool):
        raise ManifestError(path, "Expected boolean")


def _size(value, path):
    if type(value) is not int:
        raise ManifestError(path, "Expected positive integer")

    if not 1 <= value <= 9007199254740991:
        raise ManifestError(path, "Integer must be between 1 and 9007199254740991")

    return int(value)


def _array(value, path):
    if not isinstance(value, list):
        raise ManifestError(path, "Expected array")


def _strings(value, path, pattern):
    _array(value, path)
    seen = set()

    for index, item in enumerate(value):
        item_path = f"{path}[{index}]"
        _pattern(item, item_path, pattern)

        if item in seen:
            raise ManifestError(item_path, "Duplicate entry")

        seen.add(item)


def _platforms(value, path):
    _strings(value, path, r"linux|macos|windows")
    if not value:
        raise ManifestError(path, "At least one platform is required")


def _url(value, path):
    _text(value, path, True)

    if (
        not value.startswith("https://")
        or re.search(r"\s", value)
        or "#" in value
        or "\\" in value
    ):
        raise ManifestError(path, "Expected HTTPS URL without credentials or fragment")
    try:
        parsed = urlsplit(value)

        if (
            not parsed.hostname
            or parsed.username is not None
            or parsed.password is not None
        ):
            raise ValueError()
        if parsed.port is not None and not 1 <= parsed.port <= 65535:
            raise ValueError()

        if re.search(r"%(?![0-9A-Fa-f]{2})", value):
            raise ValueError()
    except ValueError as error:
        raise ManifestError(path, "Invalid HTTPS URL") from error


def _path(value, path, platforms):
    _text(value, path, True)

    if re.search(r'[\\:<>"|?*]', value):
        raise ManifestError(
            path, "Expected relative path with forward-slash separators"
        )
    for component in value.split("/"):
        if not component or component in (".", ".."):
            raise ManifestError(
                path, "Empty and traversal path components are not allowed"
            )
        if "windows" in platforms:
            stem = component.split(".", 1)[0].upper()

            if component.endswith((" ", ".")) or re.fullmatch(
                r"CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9]", stem
            ):
                raise ManifestError(path, "Path is not a valid Windows filename")


def _validate_manifest(data, expected_name):
    allowed = {
        "$schema",
        "schema_version",
        "name",
        "min_version",
        "needs_lila",
        "licence",
        "description",
        "packages",
        "manual_installs",
        "assets",
    }
    _object(data, "$", allowed, ("name", "min_version"))
    _pattern(data["name"], "$.name", r"[a-z][a-z0-9_]*")

    if expected_name is not None and data["name"] != expected_name:
        raise ManifestError("$.name", "Manifest name does not match requested module")

    _pattern(
        data["min_version"],
        "$.min_version",
        r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)",
    )

    if "schema_version" in data:
        if type(data["schema_version"]) is not int or data["schema_version"] != 1:
            raise ManifestError("$.schema_version", "Unsupported schema revision")
    elif any(key in data for key in ("packages", "manual_installs", "assets")):
        raise ManifestError(
            "$.schema_version", "Acquisition declarations require schema revision 1"
        )

    for key in ("$schema", "licence", "description"):
        if key in data:
            _text(data[key], f"$.{key}", key == "$schema")

    _boolean(data.get("needs_lila", False), "$.needs_lila")

    if "packages" in data:
        _packages(data["packages"])

    for key, validator in (("manual_installs", _manual), ("assets", _asset)):
        entries = data.get(key, [])
        _array(entries, f"$.{key}")
        seen = set()

        for index, entry in enumerate(entries):
            path = f"$.{key}[{index}]"
            validator(entry, path)

            if entry["id"] in seen:
                raise ManifestError(f"{path}.id", "Duplicate requirement ID")

            seen.add(entry["id"])

    data.setdefault("schema_version", 1)
    data.setdefault("needs_lila", False)
    data.setdefault("packages", {})
    data.setdefault("manual_installs", [])
    data.setdefault("assets", [])


def _packages(data):
    package = r"[A-Za-z0-9][A-Za-z0-9+_.-]*"
    repository = r"[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*"
    _object(data, "$.packages", ("linux", "macos", "windows"))

    if "linux" in data:
        linux = data["linux"]
        _object(linux, "$.packages.linux", ("arch", "fedora", "ubuntu"))

        for distro, fields in (
            ("arch", ("official", "aur")),
            ("fedora", ("copr", "packages")),
            ("ubuntu", ("ppa", "packages")),
        ):
            if distro not in linux:
                continue

            branch = linux[distro]
            path = f"$.packages.linux.{distro}"
            _object(branch, path, fields)

            for key, entries in branch.items():
                _strings(
                    entries,
                    f"{path}.{key}",
                    repository if key in ("copr", "ppa") else package,
                )

    if "macos" in data:
        macos = data["macos"]
        _object(macos, "$.packages.macos", ("brew",))

        if "brew" in macos:
            brew = macos["brew"]
            _object(brew, "$.packages.macos.brew", ("taps", "formulae"))

            if "taps" in brew:
                _strings(brew["taps"], "$.packages.macos.brew.taps", repository)

            if "formulae" in brew:
                _strings(
                    brew["formulae"],
                    "$.packages.macos.brew.formulae",
                    r"[a-z0-9][a-z0-9+_.@-]*(?:/[a-z0-9][a-z0-9_.-]*/[a-z0-9][a-z0-9+_.@-]*)?",
                )

    if "windows" in data:
        _windows_packages(data["windows"])


def _windows_packages(data):
    path = "$.packages.windows"
    token = r"[A-Za-z0-9][A-Za-z0-9_.-]*"
    _object(data, path, ("winget_sources", "winget", "vcpkg"))

    sources = data.get("winget_sources", [])
    _array(sources, f"{path}.winget_sources")
    names = {"winget", "msstore"}

    for index, source in enumerate(sources):
        location = f"{path}.winget_sources[{index}]"
        _object(source, location, ("name", "url", "type"), ("name", "url", "type"))
        _pattern(source["name"], f"{location}.name", token)
        name = source["name"].lower()

        if name in names:
            raise ManifestError(f"{location}.name", "Duplicate or built-in source name")

        names.add(name)
        _url(source["url"], f"{location}.url")

        if source["type"] not in ("Microsoft.Rest", "Microsoft.PreIndexed.Package"):
            raise ManifestError(f"{location}.type", "Unsupported Windows source type")

    winget = data.get("winget", [])
    _array(winget, f"{path}.winget")
    seen = set()

    for index, entry in enumerate(winget):
        location = f"{path}.winget[{index}]"
        if isinstance(entry, str):
            identifier, source = entry, "winget"
            _pattern(identifier, location, token)
        else:
            _object(entry, location, ("id", "source"), ("id", "source"))
            identifier, source = entry["id"], entry["source"]
            _pattern(identifier, f"{location}.id", token)
            _pattern(source, f"{location}.source", token)

        if source.lower() not in names:
            raise ManifestError(
                location, "Package references an undeclared Windows source"
            )

        identity = (identifier, source.lower())

        if identity in seen:
            raise ManifestError(location, "Duplicate package requirement")

        seen.add(identity)

    if "vcpkg" in data:
        _strings(data["vcpkg"], f"{path}.vcpkg", r"[a-z0-9]+(?:-[a-z0-9]+)*")


def _manual(data, path):
    fields = ("id", "platforms", "url", "description", "verify", "environment")
    _object(data, path, fields, fields[:-1])

    _pattern(data["id"], f"{path}.id", r"[a-z][a-z0-9_]*")
    _platforms(data["platforms"], f"{path}.platforms")
    _url(data["url"], f"{path}.url")
    _text(data["description"], f"{path}.description", True)

    _object(data["verify"], f"{path}.verify", ("path",), ("path",))
    _path(data["verify"]["path"], f"{path}.verify.path", data["platforms"])

    environment = data.get("environment", {})

    if not isinstance(environment, dict):
        raise ManifestError(f"{path}.environment", "Expected object")

    names = set()

    for name, value in environment.items():
        location = f"{path}.environment.{name}"
        _pattern(name, location, r"[A-Za-z_][A-Za-z0-9_]*")
        identity = name.upper() if "windows" in data["platforms"] else name

        if identity in names:
            raise ManifestError(location, "Conflicting environment variable names")

        names.add(identity)

        if isinstance(value, str):
            value = {"value": value, "operation": "set", "scope": "user"}

        _object(
            value,
            location,
            ("value", "operation", "scope"),
            ("value", "operation", "scope"),
        )
        _text(value["value"], f"{location}.value", True)

        if (
            re.fullmatch(r"(?:[^{}]|\{selected_install_directory\})*", value["value"])
            is None
        ):
            raise ManifestError(f"{location}.value", "Unsupported placeholder")

        if value["operation"] not in ("set", "append_path") or value["scope"] not in (
            "project",
            "user",
        ):
            raise ManifestError(location, "Unsupported environment operation or scope")

        environment[name] = value

    data["environment"] = environment


def _asset(data, path):
    required = (
        "id",
        "url",
        "sha256",
        "size_bytes",
        "license_url",
        "destination",
        "description",
        "archive_format",
        "max_extracted_size_bytes",
        "max_files",
    )
    _object(
        data, path, required + ("platforms", "destination_root", "optional"), required
    )

    _pattern(data["id"], f"{path}.id", r"[a-z][a-z0-9_]*")
    platforms = data.get("platforms", ["linux", "macos", "windows"])
    _platforms(platforms, f"{path}.platforms")

    for key in ("url", "license_url"):
        _url(data[key], f"{path}.{key}")

    _pattern(data["sha256"], f"{path}.sha256", r"[A-Fa-f0-9]{64}")

    for key in ("size_bytes", "max_extracted_size_bytes", "max_files"):
        data[key] = _size(data[key], f"{path}.{key}")

    _path(data["destination"], f"{path}.destination", platforms)
    _text(data["description"], f"{path}.description", True)

    if data.get("destination_root", "project") not in ("project", "module"):
        raise ManifestError(f"{path}.destination_root", "Unsupported destination root")

    if data["archive_format"] != "zip":
        raise ManifestError(f"{path}.archive_format", "Unsupported archive format")

    _boolean(data.get("optional", False), f"{path}.optional")

    data.setdefault("platforms", platforms)
    data.setdefault("destination_root", "project")
    data.setdefault("optional", False)
