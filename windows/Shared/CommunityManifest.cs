using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;

namespace Weave.Shared;

public sealed class ManifestException : Exception
{
    public string Path { get; }
    public string Reason { get; }

    public ManifestException(string path, string reason) : base($"{path}: {reason}")
    {
        Path = path;
        Reason = reason;
    }
}

public static class CommunityManifest
{
    public static JsonElement Parse(string source, string? expectedName = null)
    {
        source = source.TrimStart('\uFEFF');

        try
        {
            using var document = JsonDocument.Parse(source);
            CheckKeys(document.RootElement, "$", 0);

            var data = JsonNode.Parse(source);
            Validate(data, expectedName);

            return JsonSerializer.SerializeToElement(data);
        }
        catch (JsonException)
        {
            throw new ManifestException("$", "Invalid JSON");
        }
    }

    private static void CheckKeys(JsonElement value, string path, int depth)
    {
        if (depth >= 64 && value.ValueKind is JsonValueKind.Object or JsonValueKind.Array)
            throw new ManifestException("$", "JSON nesting exceeds supported depth");

        if (value.ValueKind == JsonValueKind.Object)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);

            foreach (var field in value.EnumerateObject())
            {
                if (!seen.Add(field.Name))
                    throw new ManifestException(path, $"Duplicate object key: {field.Name}");

                CheckKeys(field.Value, $"{path}.{field.Name}", depth + 1);
            }
        }
        else if (value.ValueKind == JsonValueKind.Array)
        {
            var index = 0;

            foreach (var item in value.EnumerateArray())
                CheckKeys(item, $"{path}[{index++}]", depth + 1);
        }
        else if (value.ValueKind == JsonValueKind.Number &&
                 value.GetRawText().IndexOfAny(['.', 'e', 'E']) >= 0)
            throw new ManifestException(path, "Expected integer JSON number");
    }

    private static JsonObject Object(JsonNode? value, string path, string[] allowed, params string[] required)
    {
        if (value is not JsonObject result)
            throw new ManifestException(path, "Expected object");

        foreach (var field in result)
        {
            if (!allowed.Contains(field.Key))
                throw new ManifestException($"{path}.{field.Key}", "Unknown field");
        }

        foreach (var key in required)
        {
            if (!result.ContainsKey(key))
                throw new ManifestException($"{path}.{key}", "Required field is missing");
        }

        return result;
    }

    private static string Text(JsonNode? value, string path, bool nonempty = false)
    {
        if (value is not JsonValue scalar ||
            !scalar.TryGetValue<string>(out var text) ||
            text == null ||
            (nonempty && text.Length == 0))
        {
            throw new ManifestException(path, nonempty ? "Expected nonempty string" : "Expected string");
        }

        if (text.Any(character => character < 32 || character == 127))
            throw new ManifestException(path, "Control characters are not allowed");

        return text;
    }

    private static string Pattern(JsonNode? value, string path, string pattern)
    {
        var text = Text(value, path, true);

        if (!Regex.IsMatch(text, $"\\A(?:{pattern})\\z", RegexOptions.CultureInvariant))
            throw new ManifestException(path, "Invalid identifier");

        return text;
    }

    private static void Boolean(JsonNode? value, string path)
    {
        if (value is not JsonValue scalar || !scalar.TryGetValue<bool>(out _))
            throw new ManifestException(path, "Expected boolean");
    }

    private static long Size(JsonNode? value, string path)
    {
        if (value == null || value.GetValueKind() != JsonValueKind.Number)
            throw new ManifestException(path, "Expected positive integer");

        if (!long.TryParse(
                value.ToJsonString(), NumberStyles.Integer, CultureInfo.InvariantCulture,
                out var number) ||
            number < 1 || number > 9007199254740991)
        {
            throw new ManifestException(path, "Integer must be between 1 and 9007199254740991");
        }

        return number;
    }

    private static JsonArray Array(JsonNode? value, string path)
    {
        if (value is not JsonArray result)
            throw new ManifestException(path, "Expected array");

        return result;
    }

    private static void Strings(JsonNode? value, string path, string pattern)
    {
        var entries = Array(value, path);
        var seen = new HashSet<string>(StringComparer.Ordinal);

        for (var index = 0; index < entries.Count; index++)
        {
            var location = $"{path}[{index}]";

            if (!seen.Add(Pattern(entries[index], location, pattern)))
                throw new ManifestException(location, "Duplicate entry");
        }
    }

    private static JsonArray Platforms(JsonNode? value, string path)
    {
        Strings(value, path, "linux|macos|windows");
        var entries = Array(value, path);

        if (entries.Count == 0)
            throw new ManifestException(path, "At least one platform is required");

        return entries;
    }

    private static void Url(JsonNode? value, string path)
    {
        var text = Text(value, path, true);

        if (!text.StartsWith("https://", StringComparison.Ordinal) ||
            text.Any(char.IsWhiteSpace) ||
            text.Contains('#') ||
            text.Contains('\\') ||
            Regex.IsMatch(text, "%(?![0-9A-Fa-f]{2})") ||
            !Uri.TryCreate(text, UriKind.Absolute, out var uri) ||
            string.IsNullOrEmpty(uri.Host) ||
            uri.UserInfo.Length > 0 ||
            uri.Port < 1 ||
            uri.Port > 65535)
        {
            throw new ManifestException(path, "Invalid HTTPS URL");
        }
    }

    private static void RelativePath(JsonNode? value, string path, JsonArray platforms)
    {
        var text = Text(value, path, true);

        if (text.IndexOfAny(['\\', ':', '<', '>', '"', '|', '?', '*']) >= 0)
            throw new ManifestException(path, "Expected relative path with forward-slash separators");

        var windows = platforms.Any(item => item?.GetValue<string>() == "windows");

        foreach (var component in text.Split('/'))
        {
            if (component.Length == 0 || component is "." or "..")
                throw new ManifestException(path, "Empty and traversal path components are not allowed");

            var stem = component.Split('.')[0].ToUpperInvariant();

            if (windows && (component.EndsWith(' ') || component.EndsWith('.') ||
                            Regex.IsMatch(stem, "\\A(?:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])\\z")))
            {
                throw new ManifestException(path, "Path is not a valid Windows filename");
            }
        }
    }

    private static void Validate(JsonNode? value, string? expectedName)
    {
        var fields = new[]
        {
            "$schema", "schema_version", "name", "min_version", "needs_lila",
            "licence", "description", "packages", "manual_installs", "assets"
        };
        var data = Object(value, "$", fields, "name", "min_version");
        var name = Pattern(data["name"], "$.name", "[a-z][a-z0-9_]*");

        if (expectedName != null && name != expectedName)
            throw new ManifestException("$.name", "Manifest name does not match requested module");

        Pattern(data["min_version"], "$.min_version", @"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)");

        if (data.ContainsKey("schema_version"))
        {
            if (Size(data["schema_version"], "$.schema_version") != 1)
                throw new ManifestException("$.schema_version", "Unsupported schema revision");
        }
        else if (new[] { "packages", "manual_installs", "assets" }.Any(data.ContainsKey))
            throw new ManifestException("$.schema_version", "Acquisition declarations require schema revision 1");

        foreach (var key in new[] { "$schema", "licence", "description" })
        {
            if (data.ContainsKey(key))
                Text(data[key], $"$.{key}", key == "$schema");
        }

        if (data.ContainsKey("needs_lila"))
            Boolean(data["needs_lila"], "$.needs_lila");

        if (data.ContainsKey("packages"))
            Packages(data["packages"]);

        foreach (var key in new[] { "manual_installs", "assets" })
        {
            if (!data.ContainsKey(key))
                continue;

            var entries = Array(data[key], $"$.{key}");
            var seen = new HashSet<string>(StringComparer.Ordinal);

            for (var index = 0; index < entries.Count; index++)
            {
                var path = $"$.{key}[{index}]";
                var entry = key == "assets" ? Asset(entries[index], path) : Manual(entries[index], path);

                if (!seen.Add(entry["id"]!.GetValue<string>()))
                    throw new ManifestException($"{path}.id", "Duplicate requirement ID");
            }
        }

        if (!data.ContainsKey("schema_version"))
            data["schema_version"] = 1;

        if (!data.ContainsKey("needs_lila"))
            data["needs_lila"] = false;

        if (!data.ContainsKey("packages"))
            data["packages"] = new JsonObject();

        if (!data.ContainsKey("manual_installs"))
            data["manual_installs"] = new JsonArray();

        if (!data.ContainsKey("assets"))
            data["assets"] = new JsonArray();
    }

    private static void Packages(JsonNode? value)
    {
        const string package = "[A-Za-z0-9][A-Za-z0-9+_.-]*";
        const string repository = "[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*";
        var data = Object(value, "$.packages", ["linux", "macos", "windows"]);

        if (data.ContainsKey("linux"))
        {
            var linux = Object(data["linux"], "$.packages.linux", ["arch", "fedora", "ubuntu"]);

            foreach (var distro in new[] { "arch", "fedora", "ubuntu" })
            {
                if (!linux.ContainsKey(distro))
                    continue;

                var path = $"$.packages.linux.{distro}";
                var allowed = distro == "arch"
                    ? new[] { "official", "aur" }
                    : distro == "fedora" ? ["copr", "packages"] : ["ppa", "packages"];
                var branch = Object(linux[distro], path, allowed);

                foreach (var field in branch)
                {
                    var pattern = field.Key is "copr" or "ppa" ? repository : package;
                    Strings(field.Value, $"{path}.{field.Key}", pattern);
                }
            }
        }

        if (data.ContainsKey("macos"))
        {
            var macos = Object(data["macos"], "$.packages.macos", ["brew"]);

            if (macos.ContainsKey("brew"))
            {
                var brew = Object(macos["brew"], "$.packages.macos.brew", ["taps", "formulae"]);

                if (brew.ContainsKey("taps"))
                    Strings(brew["taps"], "$.packages.macos.brew.taps", repository);

                if (brew.ContainsKey("formulae"))
                {
                    Strings(
                        brew["formulae"],
                        "$.packages.macos.brew.formulae",
                        "[a-z0-9][a-z0-9+_.@-]*(?:/[a-z0-9][a-z0-9_.-]*/[a-z0-9][a-z0-9+_.@-]*)?");
                }
            }
        }

        if (data.ContainsKey("windows"))
            WindowsPackages(data["windows"]);
    }

    private static void WindowsPackages(JsonNode? value)
    {
        const string path = "$.packages.windows";
        const string token = "[A-Za-z0-9][A-Za-z0-9_.-]*";
        var data = Object(value, path, ["winget_sources", "winget", "vcpkg"]);
        var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase) { "winget", "msstore" };

        if (data.ContainsKey("winget_sources"))
        {
            var sources = Array(data["winget_sources"], $"{path}.winget_sources");

            for (var index = 0; index < sources.Count; index++)
            {
                var location = $"{path}.winget_sources[{index}]";
                var source = Object(
                    sources[index], location,
                    ["name", "url", "type"], "name", "url", "type");

                if (!names.Add(Pattern(source["name"], $"{location}.name", token)))
                    throw new ManifestException($"{location}.name", "Duplicate or built-in source name");

                Url(source["url"], $"{location}.url");

                var sourceType = Text(source["type"], $"{location}.type");

                if (sourceType is not ("Microsoft.Rest" or "Microsoft.PreIndexed.Package"))
                    throw new ManifestException($"{location}.type", "Unsupported Windows source type");
            }
        }

        if (data.ContainsKey("winget"))
        {
            var entries = Array(data["winget"], $"{path}.winget");
            var seen = new HashSet<(string, string)>();

            for (var index = 0; index < entries.Count; index++)
            {
                var location = $"{path}.winget[{index}]";
                string identifier;
                string source;

                if (entries[index] is JsonValue scalar && scalar.TryGetValue<string>(out _))
                {
                    identifier = Pattern(entries[index], location, token);
                    source = "winget";
                }
                else
                {
                    var entry = Object(entries[index], location, ["id", "source"], "id", "source");
                    identifier = Pattern(entry["id"], $"{location}.id", token);
                    source = Pattern(entry["source"], $"{location}.source", token);
                }

                if (!names.Contains(source))
                    throw new ManifestException(location, "Package references an undeclared Windows source");

                if (!seen.Add((identifier, source.ToLowerInvariant())))
                    throw new ManifestException(location, "Duplicate package requirement");
            }
        }

        if (data.ContainsKey("vcpkg"))
            Strings(data["vcpkg"], $"{path}.vcpkg", "[a-z0-9]+(?:-[a-z0-9]+)*");
    }

    private static JsonObject Manual(JsonNode? value, string path)
    {
        var data = Object(
            value, path,
            ["id", "platforms", "url", "description", "verify", "environment"],
            "id", "platforms", "url", "description", "verify");

        Pattern(data["id"], $"{path}.id", "[a-z][a-z0-9_]*");
        var platforms = Platforms(data["platforms"], $"{path}.platforms");

        Url(data["url"], $"{path}.url");
        Text(data["description"], $"{path}.description", true);

        var verify = Object(data["verify"], $"{path}.verify", ["path"], "path");
        RelativePath(verify["path"], $"{path}.verify.path", platforms);

        if (!data.ContainsKey("environment"))
            data["environment"] = new JsonObject();

        if (data["environment"] is not JsonObject environment)
            throw new ManifestException($"{path}.environment", "Expected object");

        var includesWindows = platforms.Any(item => item?.GetValue<string>() == "windows");
        var comparer = includesWindows ? StringComparer.OrdinalIgnoreCase : StringComparer.Ordinal;
        var names = new HashSet<string>(comparer);

        foreach (var name in environment.Select(field => field.Key).ToArray())
        {
            var location = $"{path}.environment.{name}";
            Pattern(JsonValue.Create(name), location, "[A-Za-z_][A-Za-z0-9_]*");

            if (!names.Add(name))
                throw new ManifestException(location, "Conflicting environment variable names");

            if (environment[name] is JsonValue scalar && scalar.TryGetValue<string>(out var shorthand))
            {
                environment[name] = new JsonObject
                {
                    ["value"] = shorthand,
                    ["operation"] = "set",
                    ["scope"] = "user"
                };
            }

            var change = Object(
                environment[name], location,
                ["value", "operation", "scope"],
                "value", "operation", "scope");
            var text = Text(change["value"], $"{location}.value", true);

            if (!Regex.IsMatch(text, @"\A(?:[^{}]|\{selected_install_directory\})*\z"))
                throw new ManifestException($"{location}.value", "Unsupported placeholder");

            if (Text(change["operation"], $"{location}.operation") is not ("set" or "append_path") ||
                Text(change["scope"], $"{location}.scope") is not ("project" or "user"))
            {
                throw new ManifestException(location, "Unsupported environment operation or scope");
            }
        }

        return data;
    }

    private static JsonObject Asset(JsonNode? value, string path)
    {
        var required = new[]
        {
            "id", "url", "sha256", "size_bytes", "license_url", "destination",
            "description", "archive_format", "max_extracted_size_bytes", "max_files"
        };
        var allowed = required.Concat(["platforms", "destination_root", "optional"]).ToArray();
        var data = Object(value, path, allowed, required);

        Pattern(data["id"], $"{path}.id", "[a-z][a-z0-9_]*");

        if (!data.ContainsKey("platforms"))
            data["platforms"] = new JsonArray("linux", "macos", "windows");

        var platforms = Platforms(data["platforms"], $"{path}.platforms");

        Url(data["url"], $"{path}.url");
        Url(data["license_url"], $"{path}.license_url");
        Pattern(data["sha256"], $"{path}.sha256", "[A-Fa-f0-9]{64}");

        foreach (var key in new[] { "size_bytes", "max_extracted_size_bytes", "max_files" })
            data[key] = Size(data[key], $"{path}.{key}");

        RelativePath(data["destination"], $"{path}.destination", platforms);
        Text(data["description"], $"{path}.description", true);

        if (!data.ContainsKey("destination_root"))
            data["destination_root"] = "project";

        if (Text(data["destination_root"], $"{path}.destination_root") is not ("project" or "module"))
            throw new ManifestException($"{path}.destination_root", "Unsupported destination root");

        if (Text(data["archive_format"], $"{path}.archive_format") != "zip")
            throw new ManifestException($"{path}.archive_format", "Unsupported archive format");

        if (!data.ContainsKey("optional"))
            data["optional"] = false;

        Boolean(data["optional"], $"{path}.optional");

        return data;
    }
}
