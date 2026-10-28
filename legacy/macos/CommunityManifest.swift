import CoreFoundation
import Foundation

struct ManifestError: Error, LocalizedError {
    let path: String
    let reason: String

    var errorDescription: String? { "\(path): \(reason)" }
}

enum CommunityManifest {
    static func parse(_ source: String, expectedName: String? = nil) throws -> [String: Any] {
        var text = source
        while text.hasPrefix("\u{FEFF}") { text.removeFirst() }

        let bytes = Data(text.utf8)
        let decoded: Any

        do {
            decoded = try JSONSerialization.jsonObject(with: bytes, options: [.fragmentsAllowed])
        } catch {
            throw ManifestError(path: "$", reason: "Invalid JSON")
        }

        var scanner = KeyScanner(bytes: Array(bytes))
        try scanner.value(path: "$", depth: 0)
        scanner.whitespace()

        guard scanner.index == scanner.bytes.count else {
            throw ManifestError(path: "$", reason: "Invalid JSON")
        }

        try checkNumbers(decoded, path: "$")
        return try validate(decoded, expectedName: expectedName)
    }

    private struct KeyScanner {
        let bytes: [UInt8]
        var index = 0

        mutating func whitespace() {
            while index < bytes.count && [9, 10, 13, 32].contains(bytes[index]) {
                index += 1
            }
        }

        mutating func string() throws -> String {
            let start = index
            index += 1

            while index < bytes.count {
                if bytes[index] == 92 {
                    index += 2
                } else if bytes[index] == 34 {
                    index += 1
                    let token = Data(bytes[start..<index])
                    guard
                        let result = try JSONSerialization.jsonObject(
                            with: token, options: [.fragmentsAllowed]) as? String
                    else {
                        throw ManifestError(path: "$", reason: "Invalid JSON")
                    }

                    return result
                } else {
                    index += 1
                }
            }

            throw ManifestError(path: "$", reason: "Invalid JSON")
        }

        mutating func value(path: String, depth: Int) throws {
            whitespace()

            guard index < bytes.count else {
                throw ManifestError(path: "$", reason: "Invalid JSON")
            }

            if depth >= 64 && [91, 123].contains(bytes[index]) {
                throw ManifestError(path: "$", reason: "JSON nesting exceeds supported depth")
            }

            switch bytes[index] {
            case 123:
                index += 1
                whitespace()
                var names = Set<String>()

                while index < bytes.count && bytes[index] != 125 {
                    guard bytes[index] == 34 else {
                        throw ManifestError(path: "$", reason: "Invalid JSON")
                    }
                    let name = try string()
                    guard names.insert(name).inserted else {
                        throw ManifestError(path: path, reason: "Duplicate object key: \(name)")
                    }

                    whitespace()
                    guard index < bytes.count && bytes[index] == 58 else {
                        throw ManifestError(path: "$", reason: "Invalid JSON")
                    }
                    index += 1
                    try value(path: "\(path).\(name)", depth: depth + 1)
                    whitespace()

                    if index < bytes.count && bytes[index] == 44 {
                        index += 1
                        whitespace()
                        guard index < bytes.count && bytes[index] == 34 else {
                            throw ManifestError(path: "$", reason: "Invalid JSON")
                        }
                    } else if index >= bytes.count || bytes[index] != 125 {
                        throw ManifestError(path: "$", reason: "Invalid JSON")
                    }
                }

                guard index < bytes.count else {
                    throw ManifestError(path: "$", reason: "Invalid JSON")
                }
                index += 1
            case 91:
                index += 1
                whitespace()
                var item = 0

                while index < bytes.count && bytes[index] != 93 {
                    try value(path: "\(path)[\(item)]", depth: depth + 1)
                    item += 1
                    whitespace()

                    if index < bytes.count && bytes[index] == 44 {
                        index += 1
                        whitespace()
                        guard index < bytes.count && bytes[index] != 93 else {
                            throw ManifestError(path: "$", reason: "Invalid JSON")
                        }
                    } else if index >= bytes.count || bytes[index] != 93 {
                        throw ManifestError(path: "$", reason: "Invalid JSON")
                    }
                }

                guard index < bytes.count else {
                    throw ManifestError(path: "$", reason: "Invalid JSON")
                }
                index += 1
            case 34:
                _ = try string()
            default:
                let start = index

                while index < bytes.count && ![9, 10, 13, 32, 44, 93, 125].contains(bytes[index]) {
                    index += 1
                }

                let token = String(decoding: bytes[start..<index], as: UTF8.self)

                if bytes[start] == 45 || (48...57).contains(bytes[start]) {
                    if token.contains(".") || token.contains("e") || token.contains("E") {
                        throw ManifestError(path: path, reason: "Expected integer JSON number")
                    }
                    guard
                        token.range(of: "\\A-?(?:0|[1-9][0-9]*)\\z", options: .regularExpression)
                            != nil
                    else {
                        throw ManifestError(path: "$", reason: "Invalid JSON")
                    }
                } else if !["true", "false", "null"].contains(token) {
                    throw ManifestError(path: "$", reason: "Invalid JSON")
                }
            }
        }
    }

    private static func checkNumbers(_ value: Any, path: String) throws {
        if let dictionary = value as? [String: Any] {
            for (key, child) in dictionary {
                try checkNumbers(child, path: "\(path).\(key)")
            }
        } else if let array = value as? [Any] {
            for (index, child) in array.enumerated() {
                try checkNumbers(child, path: "\(path)[\(index)]")
            }
        } else if let number = value as? NSNumber, !number.doubleValue.isFinite {
            throw ManifestError(path: path, reason: "Nonstandard or nonfinite number")
        }
    }

    private static func object(
        _ value: Any?, path: String, allowed: [String], required: [String] = []
    ) throws -> [String: Any] {
        guard let result = value as? [String: Any] else {
            throw ManifestError(path: path, reason: "Expected object")
        }
        for key in result.keys where !allowed.contains(key) {
            throw ManifestError(path: "\(path).\(key)", reason: "Unknown field")
        }
        for key in required where result[key] == nil {
            throw ManifestError(path: "\(path).\(key)", reason: "Required field is missing")
        }
        return result
    }

    private static func text(_ value: Any?, path: String, nonempty: Bool = false) throws -> String {
        guard let result = value as? String, !nonempty || !result.isEmpty else {
            throw ManifestError(
                path: path, reason: nonempty ? "Expected nonempty string" : "Expected string")
        }
        guard !result.unicodeScalars.contains(where: { $0.value < 32 || $0.value == 127 }) else {
            throw ManifestError(path: path, reason: "Control characters are not allowed")
        }
        return result
    }

    private static func pattern(_ value: Any?, path: String, expression: String) throws -> String {
        let result = try text(value, path: path, nonempty: true)
        let regex = try NSRegularExpression(pattern: "\\A(?:\(expression))\\z")
        guard regex.firstMatch(in: result, range: NSRange(result.startIndex..., in: result)) != nil
        else {
            throw ManifestError(path: path, reason: "Invalid identifier")
        }
        return result
    }

    private static func boolean(_ value: Any?, path: String) throws {
        guard let number = value as? NSNumber, CFGetTypeID(number) == CFBooleanGetTypeID() else {
            throw ManifestError(path: path, reason: "Expected boolean")
        }
    }

    private static func size(_ value: Any?, path: String) throws -> Int64 {
        guard let number = value as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID(),
            number.doubleValue.isFinite,
            number.doubleValue.rounded(.towardZero) == number.doubleValue
        else {
            throw ManifestError(path: path, reason: "Expected positive integer")
        }
        guard number.doubleValue >= 1 && number.doubleValue <= 9_007_199_254_740_991 else {
            throw ManifestError(
                path: path, reason: "Integer must be between 1 and 9007199254740991")
        }
        return number.int64Value
    }

    private static func array(_ value: Any?, path: String) throws -> [Any] {
        guard let result = value as? [Any] else {
            throw ManifestError(path: path, reason: "Expected array")
        }
        return result
    }

    private static func strings(_ value: Any?, path: String, expression: String) throws -> [String]
    {
        let entries = try array(value, path: path)
        var seen = Set<String>()
        var result: [String] = []
        for (index, item) in entries.enumerated() {
            let location = "\(path)[\(index)]"
            let entry = try pattern(item, path: location, expression: expression)
            guard seen.insert(entry).inserted else {
                throw ManifestError(path: location, reason: "Duplicate entry")
            }
            result.append(entry)
        }
        return result
    }

    private static func platforms(_ value: Any?, path: String) throws -> [String] {
        let result = try strings(value, path: path, expression: "linux|macos|windows")
        guard !result.isEmpty else {
            throw ManifestError(path: path, reason: "At least one platform is required")
        }
        return result
    }

    private static func url(_ value: Any?, path: String) throws {
        let result = try text(value, path: path, nonempty: true)
        let invalidEscape = try NSRegularExpression(pattern: "%(?![0-9A-Fa-f]{2})")
        guard result.hasPrefix("https://"), !result.contains("#"), !result.contains("\\"),
            !result.unicodeScalars.contains(where: {
                CharacterSet.whitespacesAndNewlines.contains($0)
            }),
            invalidEscape.firstMatch(in: result, range: NSRange(result.startIndex..., in: result))
                == nil,
            let parsed = URLComponents(string: result), let host = parsed.host, !host.isEmpty,
            parsed.user == nil, parsed.password == nil,
            parsed.port == nil || (1...65535).contains(parsed.port ?? 0)
        else {
            throw ManifestError(path: path, reason: "Invalid HTTPS URL")
        }
    }

    private static func relativePath(_ value: Any?, path: String, platforms: [String]) throws {
        let result = try text(value, path: path, nonempty: true)
        guard !result.contains(where: { "\\:<>\"|?*".contains($0) }) else {
            throw ManifestError(
                path: path, reason: "Expected relative path with forward-slash separators")
        }
        for component in result.components(separatedBy: "/") {
            guard !component.isEmpty, component != ".", component != ".." else {
                throw ManifestError(
                    path: path, reason: "Empty and traversal path components are not allowed")
            }
            let stem = component.components(separatedBy: ".")[0].uppercased()
            let reserved =
                ["CON", "PRN", "AUX", "NUL"] + (1...9).flatMap { ["COM\($0)", "LPT\($0)"] }
            if platforms.contains("windows")
                && (component.hasSuffix(" ") || component.hasSuffix(".") || reserved.contains(stem))
            {
                throw ManifestError(path: path, reason: "Path is not a valid Windows filename")
            }
        }
    }

    private static func validate(_ value: Any, expectedName: String?) throws -> [String: Any] {
        let fields = [
            "$schema", "schema_version", "name", "min_version", "needs_lila", "licence",
            "description", "packages", "manual_installs", "assets",
        ]
        var data = try object(value, path: "$", allowed: fields, required: ["name", "min_version"])
        let name = try pattern(data["name"], path: "$.name", expression: "[a-z][a-z0-9_]*")

        if let expected = expectedName, expected != name {
            throw ManifestError(
                path: "$.name", reason: "Manifest name does not match requested module")
        }

        _ = try pattern(
            data["min_version"], path: "$.min_version",
            expression: "(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)")

        if let revision = data["schema_version"] {
            guard try size(revision, path: "$.schema_version") == 1 else {
                throw ManifestError(path: "$.schema_version", reason: "Unsupported schema revision")
            }
        } else if ["packages", "manual_installs", "assets"].contains(where: { data[$0] != nil }) {
            throw ManifestError(
                path: "$.schema_version",
                reason: "Acquisition declarations require schema revision 1")
        }

        for key in ["$schema", "licence", "description"] {
            if let field = data[key] {
                _ = try text(field, path: "$.\(key)", nonempty: key == "$schema")
            }
        }

        try boolean(data["needs_lila"] ?? false, path: "$.needs_lila")

        if let packages = data["packages"] {
            try validatePackages(packages)
        }

        for key in ["manual_installs", "assets"] {
            let entries = try array(data[key] ?? [Any](), path: "$.\(key)")
            var seen = Set<String>()
            var normalized: [[String: Any]] = []

            for (index, entry) in entries.enumerated() {
                let path = "$.\(key)[\(index)]"
                let requirement: [String: Any]
                if key == "assets" {
                    requirement = try asset(entry, path: path)
                } else {
                    requirement = try manual(entry, path: path)
                }

                let id = try text(requirement["id"], path: "\(path).id")

                guard seen.insert(id).inserted else {
                    throw ManifestError(path: "\(path).id", reason: "Duplicate requirement ID")
                }

                normalized.append(requirement)
            }

            data[key] = normalized
        }

        data["schema_version"] = 1
        data["needs_lila"] = data["needs_lila"] ?? false
        data["packages"] = data["packages"] ?? [String: Any]()
        return data
    }

    private static func validatePackages(_ value: Any) throws {
        let package = "[A-Za-z0-9][A-Za-z0-9+_.-]*"
        let repository = "[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*"
        let data = try object(value, path: "$.packages", allowed: ["linux", "macos", "windows"])

        if let linuxValue = data["linux"] {
            let linux = try object(
                linuxValue, path: "$.packages.linux", allowed: ["arch", "fedora", "ubuntu"])

            for (distro, fields) in [
                ("arch", ["official", "aur"]), ("fedora", ["copr", "packages"]),
                ("ubuntu", ["ppa", "packages"]),
            ] {
                guard let value = linux[distro] else { continue }

                let path = "$.packages.linux.\(distro)"
                let branch = try object(value, path: path, allowed: fields)

                for (key, entries) in branch {
                    _ = try strings(
                        entries, path: "\(path).\(key)",
                        expression: ["copr", "ppa"].contains(key) ? repository : package)
                }
            }
        }

        if let macosValue = data["macos"] {
            let macos = try object(macosValue, path: "$.packages.macos", allowed: ["brew"])
            if let brewValue = macos["brew"] {
                let brew = try object(
                    brewValue, path: "$.packages.macos.brew", allowed: ["taps", "formulae"])
                if let taps = brew["taps"] {
                    _ = try strings(
                        taps, path: "$.packages.macos.brew.taps", expression: repository)
                }
                if let formulae = brew["formulae"] {
                    _ = try strings(
                        formulae, path: "$.packages.macos.brew.formulae",
                        expression:
                            "[a-z0-9][a-z0-9+_.@-]*(?:/[a-z0-9][a-z0-9_.-]*/[a-z0-9][a-z0-9+_.@-]*)?"
                    )
                }
            }
        }

        if let windows = data["windows"] {
            try windowsPackages(windows)
        }
    }

    private static func windowsPackages(_ value: Any) throws {
        let path = "$.packages.windows"
        let token = "[A-Za-z0-9][A-Za-z0-9_.-]*"
        let data = try object(value, path: path, allowed: ["winget_sources", "winget", "vcpkg"])
        var names: Set<String> = ["winget", "msstore"]
        let sources = try array(data["winget_sources"] ?? [Any](), path: "\(path).winget_sources")

        for (index, value) in sources.enumerated() {
            let location = "\(path).winget_sources[\(index)]"
            let source = try object(
                value, path: location, allowed: ["name", "url", "type"],
                required: ["name", "url", "type"])
            let name = try pattern(source["name"], path: "\(location).name", expression: token)
                .lowercased()

            guard names.insert(name).inserted else {
                throw ManifestError(
                    path: "\(location).name", reason: "Duplicate or built-in source name")
            }

            try url(source["url"], path: "\(location).url")
            let type = try text(source["type"], path: "\(location).type")

            guard ["Microsoft.Rest", "Microsoft.PreIndexed.Package"].contains(type) else {
                throw ManifestError(
                    path: "\(location).type", reason: "Unsupported Windows source type")
            }
        }

        let entries = try array(data["winget"] ?? [Any](), path: "\(path).winget")
        var seen = Set<String>()

        for (index, value) in entries.enumerated() {
            let location = "\(path).winget[\(index)]"
            let identifier: String
            let source: String
            if value is String {
                identifier = try pattern(value, path: location, expression: token)
                source = "winget"
            } else {
                let entry = try object(
                    value, path: location, allowed: ["id", "source"], required: ["id", "source"])
                identifier = try pattern(entry["id"], path: "\(location).id", expression: token)
                source = try pattern(entry["source"], path: "\(location).source", expression: token)
                    .lowercased()
            }

            guard names.contains(source) else {
                throw ManifestError(
                    path: location, reason: "Package references an undeclared Windows source")
            }

            guard seen.insert("\(source)/\(identifier)").inserted else {
                throw ManifestError(path: location, reason: "Duplicate package requirement")
            }
        }

        if let ports = data["vcpkg"] {
            _ = try strings(ports, path: "\(path).vcpkg", expression: "[a-z0-9]+(?:-[a-z0-9]+)*")
        }
    }

    private static func manual(_ value: Any, path: String) throws -> [String: Any] {
        let required = ["id", "platforms", "url", "description", "verify"]
        var data = try object(
            value, path: path, allowed: required + ["environment"], required: required)

        _ = try pattern(data["id"], path: "\(path).id", expression: "[a-z][a-z0-9_]*")
        let platforms = try Self.platforms(data["platforms"], path: "\(path).platforms")
        try url(data["url"], path: "\(path).url")
        _ = try text(data["description"], path: "\(path).description", nonempty: true)

        let verify = try object(
            data["verify"], path: "\(path).verify", allowed: ["path"], required: ["path"])
        try relativePath(verify["path"], path: "\(path).verify.path", platforms: platforms)

        guard let environment = (data["environment"] ?? [String: Any]()) as? [String: Any] else {
            throw ManifestError(path: "\(path).environment", reason: "Expected object")
        }
        var names = Set<String>()
        var normalized: [String: Any] = [:]

        for (name, value) in environment {
            let location = "\(path).environment.\(name)"
            _ = try pattern(name, path: location, expression: "[A-Za-z_][A-Za-z0-9_]*")
            let identity = platforms.contains("windows") ? name.uppercased() : name

            guard names.insert(identity).inserted else {
                throw ManifestError(
                    path: location, reason: "Conflicting environment variable names")
            }

            let raw: Any =
                value is String ? ["value": value, "operation": "set", "scope": "user"] : value
            let change = try object(
                raw, path: location, allowed: ["value", "operation", "scope"],
                required: ["value", "operation", "scope"])
            let text = try Self.text(change["value"], path: "\(location).value", nonempty: true)

            let regex = try NSRegularExpression(
                pattern: "\\A(?:[^{}]|\\{selected_install_directory\\})*\\z")
            guard regex.firstMatch(in: text, range: NSRange(text.startIndex..., in: text)) != nil
            else {
                throw ManifestError(path: "\(location).value", reason: "Unsupported placeholder")
            }

            let operation = try self.text(change["operation"], path: "\(location).operation")
            let scope = try self.text(change["scope"], path: "\(location).scope")

            guard ["set", "append_path"].contains(operation), ["project", "user"].contains(scope)
            else {
                throw ManifestError(
                    path: location, reason: "Unsupported environment operation or scope")
            }

            normalized[name] = change
        }

        data["environment"] = normalized
        return data
    }

    private static func asset(_ value: Any, path: String) throws -> [String: Any] {
        let required = [
            "id", "url", "sha256", "size_bytes", "license_url", "destination", "description",
            "archive_format", "max_extracted_size_bytes", "max_files",
        ]
        var data = try object(
            value, path: path, allowed: required + ["platforms", "destination_root", "optional"],
            required: required)

        _ = try pattern(data["id"], path: "\(path).id", expression: "[a-z][a-z0-9_]*")
        let platforms = try Self.platforms(
            data["platforms"] ?? ["linux", "macos", "windows"], path: "\(path).platforms")

        try url(data["url"], path: "\(path).url")
        try url(data["license_url"], path: "\(path).license_url")
        _ = try pattern(data["sha256"], path: "\(path).sha256", expression: "[A-Fa-f0-9]{64}")

        for key in ["size_bytes", "max_extracted_size_bytes", "max_files"] {
            data[key] = try size(data[key], path: "\(path).\(key)")
        }

        try relativePath(data["destination"], path: "\(path).destination", platforms: platforms)
        _ = try text(data["description"], path: "\(path).description", nonempty: true)
        let root = try text(data["destination_root"] ?? "project", path: "\(path).destination_root")

        guard ["project", "module"].contains(root) else {
            throw ManifestError(
                path: "\(path).destination_root", reason: "Unsupported destination root")
        }

        guard try text(data["archive_format"], path: "\(path).archive_format") == "zip" else {
            throw ManifestError(
                path: "\(path).archive_format", reason: "Unsupported archive format")
        }

        try boolean(data["optional"] ?? false, path: "\(path).optional")

        data["platforms"] = platforms
        data["destination_root"] = root
        data["optional"] = data["optional"] ?? false
        return data
    }
}
