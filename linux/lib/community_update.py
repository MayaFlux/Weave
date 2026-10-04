import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from urllib.parse import urlsplit
from urllib.request import urlopen

from .community_manifest import ManifestError, parse_manifest


REGISTRY_URL = "https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json"
NAME = re.compile(r"[a-z][a-z0-9_]*")
VERSION = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
VERSION_FILE = re.compile(r'set\(PACKAGE_VERSION\s+"([^"]+)"\)')


class UpdateError(Exception):
    pass


def parse_names(values):
    names = []
    seen = set()

    for value in values:
        for part in value.split(","):
            name = part.strip()
            if not name:
                raise UpdateError("Empty module name in request")
            if not NAME.fullmatch(name):
                raise UpdateError(f"Invalid module name: {name}")
            if name not in seen:
                seen.add(name)
                names.append(name)

    if not names:
        raise UpdateError("At least one module name is required")

    return names


def parse_version(value, label):
    match = VERSION.fullmatch(value)
    if not match:
        raise UpdateError(f"Malformed {label} version: {value}")
    return tuple(int(part) for part in match.groups())


def installed_version(root):
    roots = [Path(root).expanduser()] if root else [Path("/usr/local"), Path("/usr")]
    checked = []

    for prefix in roots:
        for library in ("lib", "lib64"):
            path = prefix / library / "cmake" / "MayaFlux" / "MayaFluxConfigVersion.cmake"
            checked.append(str(path))
            if not path.is_file():
                continue

            match = VERSION_FILE.search(path.read_text(encoding="utf-8"))
            if not match:
                raise UpdateError(f"Malformed MayaFlux version file: {path}")

            text = match.group(1)
            return text, parse_version(text, "installed MayaFlux")

    raise UpdateError(f"MayaFlux version file not found. Checked: {', '.join(checked)}")


def fetch_registry():
    with urlopen(REGISTRY_URL, timeout=30) as response:
        data = json.loads(response.read().decode("utf-8"))

    if not isinstance(data, list):
        raise UpdateError("Community registry must be an array")

    entries = {}
    for item in data:
        if not isinstance(item, dict):
            raise UpdateError("Community registry contains an invalid entry")

        name = item.get("name")
        repo = item.get("repo")
        minimum = item.get("min_version")
        if not isinstance(name, str) or not NAME.fullmatch(name):
            raise UpdateError("Community registry contains an invalid module name")
        if not isinstance(repo, str):
            raise UpdateError(f"Invalid repository for {name}")
        url = urlsplit(repo)
        if url.scheme != "https" or not url.hostname:
            raise UpdateError(f"Invalid repository for {name}")
        if not isinstance(minimum, str):
            raise UpdateError(f"Missing minimum version for {name}")

        parse_version(minimum, f"registry entry for {name}")
        if name in entries:
            raise UpdateError(f"Duplicate registry entry for {name}")
        entries[name] = (repo, minimum)

    return entries


def clone_module(repo, destination, report):
    process = subprocess.Popen(
        ["git", "clone", "--progress", "--depth=1", repo, str(destination)],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )

    for line in process.stdout:
        report(line.rstrip("\n"))

    if process.wait() != 0:
        raise UpdateError(f"Git clone failed for {destination.name}")


def register(path, names):
    current = path.read_text(encoding="utf-8") if path.exists() else ""
    existing = {line.strip() for line in current.splitlines() if line.strip()}
    additions = [name for name in names if name not in existing]
    if not additions:
        return []

    updated = current
    if updated and not updated.endswith("\n"):
        updated += "\n"
    updated += "".join(f"{name}\n" for name in additions)

    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", dir=path.parent, delete=False
        ) as output:
            temporary = Path(output.name)
            output.write(updated)

        mode = path.stat().st_mode & 0o777 if path.exists() else 0o644
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()

    return additions


def update(project_path, values, root, report):
    names = parse_names(values)
    project = Path(project_path).expanduser().resolve()
    if not (project / "CMakeLists.txt").is_file():
        raise UpdateError(f"Not a MayaFlux project: {project}")

    report("Fetching registry...")
    try:
        entries = fetch_registry()
        installed_text, installed = installed_version(root)
    except (OSError, UnicodeError, ValueError) as error:
        raise UpdateError(f"Could not prepare community modules: {error}") from error

    community = project / "community"
    cloned = []

    try:
        for name in names:
            if name not in entries:
                raise UpdateError(f"Module {name} is not in the community registry")

        for name in names:
            repo, registry_minimum = entries[name]
            module = community / name
            report(f"Preparing {name}...")

            if module.exists() and not module.is_dir():
                raise UpdateError(f"Community path for {name} is not a directory: {module}")

            if not module.is_dir():
                community.mkdir(exist_ok=True)
                report(f"Cloning {repo} into {module}")
                try:
                    clone_module(repo, module, report)
                finally:
                    if module.exists():
                        cloned.append(name)
            else:
                report(f"Using existing community/{name}")

            if not (module / f"{name}.cmake").is_file():
                raise UpdateError(f"Module {name} is missing {name}.cmake")
            if not (module / "src").is_dir():
                raise UpdateError(f"Module {name} is missing src/")

            manifest_path = module / "community.json"
            if not manifest_path.is_file():
                raise UpdateError(f"Module {name} is missing community.json")

            try:
                manifest = parse_manifest(
                    manifest_path.read_text(encoding="utf-8"), expected_name=name
                )
            except (OSError, UnicodeError, ManifestError) as error:
                raise UpdateError(f"Invalid manifest for {name}: {error}") from error

            required = max(
                parse_version(registry_minimum, f"registry entry for {name}"),
                parse_version(manifest["min_version"], f"manifest for {name}"),
            )
            if installed < required:
                required_text = ".".join(str(part) for part in required)
                raise UpdateError(
                    f"Module {name} requires MayaFlux >= {required_text}; found {installed_text}"
                )

            if any(manifest[key] for key in ("packages", "manual_installs", "assets")):
                report(f"{name} has declared requirements pending review")
            report(f"Validated {name}")

        added = register(project / "community.cmake", names)
    except (OSError, UpdateError) as error:
        remaining = ", ".join(cloned) if cloned else "none"
        raise UpdateError(f"{error}. Clones left in community/: {remaining}") from error

    for name in names:
        report(
            f"Registered {name} in community.cmake"
            if name in added else f"{name} was already registered"
        )

    report("Done. Rebuild the project to include the validated modules.")
    return names
