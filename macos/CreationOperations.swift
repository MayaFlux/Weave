import Foundation

enum ProjectCreation {
    static func arguments(
        name: String, location: String, withLila: Bool, copyShaderSources: Bool = false
    ) -> [String] {
        var result = ["new", name, location]
        if withLila { result.append("--with-lila") }
        if copyShaderSources { result.append("--copy-shader-sources") }
        return result
    }

    static func resolveExecutable(
        installed: URL, bundled: URL, fileExists: (String) -> Bool
    ) -> URL? {
        if fileExists(installed.path) { return installed }
        if fileExists(bundled.path) { return bundled }
        return nil
    }
}

enum CommunityModuleCreation {
    static func metadata(
        source: String,
        expectedName: String,
        description: String,
        minVersion: String,
        needsLila: Bool
    ) throws -> [String: Any] {
        var value = try CommunityManifest.parse(source, expectedName: expectedName)
        if !description.isEmpty { value["description"] = description }
        if !minVersion.isEmpty { value["min_version"] = minVersion }
        value["needs_lila"] = needsLila
        let candidate = try JSONSerialization.data(withJSONObject: value)
        return try CommunityManifest.parse(
            String(decoding: candidate, as: UTF8.self), expectedName: expectedName)
    }

    static func apply(
        source: String,
        expectedName: String,
        description: String,
        minVersion: String,
        needsLila: Bool,
        moduleDirectory: URL
    ) throws {
        let value = try metadata(
            source: source,
            expectedName: expectedName,
            description: description,
            minVersion: minVersion,
            needsLila: needsLila)
        guard let minimumVersion = value["min_version"] as? String else {
            throw ManifestError(path: "$.min_version", reason: "Expected string")
        }

        let manifestURL = moduleDirectory.appendingPathComponent("community.json")
        let cmakeURL = moduleDirectory.appendingPathComponent("\(expectedName).cmake")
        let originalCMake = try String(contentsOf: cmakeURL, encoding: .utf8)
        let versionPattern = "set\\(MF_MIN_VERSION \\\"[^\\\"\\n]+\\\"\\)"
        let lilaPattern = "set\\(MF_NEEDS_LILA (?:ON|OFF)\\)"
        let updatedCMake = try replacingExactlyOne(
            in: originalCMake,
            pattern: versionPattern,
            with: "set(MF_MIN_VERSION \"\(minimumVersion)\")")
        let finalCMake = try replacingExactlyOne(
            in: updatedCMake, pattern: lilaPattern, with: "set(MF_NEEDS_LILA \(needsLila ? "ON" : "OFF"))")
        let output = try JSONSerialization.data(withJSONObject: value, options: .prettyPrinted)

        try finalCMake.write(to: cmakeURL, atomically: true, encoding: .utf8)
        do {
            try output.write(to: manifestURL, options: .atomic)
        } catch {
            try? originalCMake.write(to: cmakeURL, atomically: true, encoding: .utf8)
            throw error
        }
    }

    private static func replacingExactlyOne(
        in source: String, pattern: String, with replacement: String
    ) throws -> String {
        let expression = try NSRegularExpression(pattern: pattern)
        let range = NSRange(source.startIndex..., in: source)
        let matches = expression.matches(in: source, range: range)
        guard matches.count == 1 else {
            throw ManifestError(path: "$", reason: "Unexpected generated CMake template")
        }
        return expression.stringByReplacingMatches(
            in: source, range: range, withTemplate: NSRegularExpression.escapedTemplate(for: replacement))
    }
}
