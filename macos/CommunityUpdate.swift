import Foundation

struct CommunityUpdateError: Error, LocalizedError {
    let message: String

    var errorDescription: String? { message }
}

enum CommunityUpdate {
    struct Version: Comparable {
        let parts: [Int]

        static func < (left: Version, right: Version) -> Bool {
            for index in 0..<3 {
                if left.parts[index] != right.parts[index] {
                    return left.parts[index] < right.parts[index]
                }
            }
            return false
        }
    }

    struct RegistryEntry {
        let repo: String
        let minimum: Version
    }

    static func names(_ values: [String]) throws -> [String] {
        var result: [String] = []
        var seen = Set<String>()

        for value in values {
            for part in value.split(separator: ",", omittingEmptySubsequences: false) {
                let name = part.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !name.isEmpty else {
                    throw CommunityUpdateError(message: "Empty module name in request")
                }
                guard name.range(of: "\\A[a-z][a-z0-9_]*\\z", options: .regularExpression) != nil else {
                    throw CommunityUpdateError(message: "Invalid module name: \(name)")
                }
                if seen.insert(name).inserted {
                    result.append(name)
                }
            }
        }

        guard !result.isEmpty else {
            throw CommunityUpdateError(message: "At least one module name is required")
        }
        return result
    }

    private static func version(_ text: String, label: String) throws -> Version {
        let pattern = "\\A(?:0|[1-9][0-9]*)\\.(?:0|[1-9][0-9]*)\\.(?:0|[1-9][0-9]*)\\z"
        guard text.range(of: pattern, options: .regularExpression) != nil else {
            throw CommunityUpdateError(message: "Malformed \(label) version: \(text)")
        }

        let parts = text.split(separator: ".").compactMap { Int($0) }
        guard parts.count == 3 else {
            throw CommunityUpdateError(message: "Malformed \(label) version: \(text)")
        }
        return Version(parts: parts)
    }

    private static func installedVersion() throws -> (String, Version) {
        let environment = ProcessInfo.processInfo.environment
        let roots: [String]

        if let explicit = environment["MAYAFLUX_ROOT"], !explicit.isEmpty {
            roots = [explicit]
        } else {
            let pathDirectories = (environment["PATH"] ?? "")
                .split(separator: ":").map(String.init)
            let optDirectories = (try? FileManager.default.contentsOfDirectory(atPath: "/opt"))?
                .map { "/opt/\($0)/bin" } ?? []
            guard let brew = (pathDirectories + optDirectories)
                .filter({ !$0.hasPrefix("/usr/local") })
                .map({ $0 + "/brew" })
                .first(where: { FileManager.default.isExecutableFile(atPath: $0) }) else {
                throw CommunityUpdateError(message: "MAYAFLUX_ROOT is unset and Homebrew was not found")
            }

            var prefixes: [String] = []
            for formula in ["mayaflux", "mayaflux-dev"] {
                let process = Process()
                process.executableURL = URL(fileURLWithPath: brew)
                process.arguments = ["--prefix", formula]
                let pipe = Pipe()
                process.standardOutput = pipe
                process.standardError = FileHandle.nullDevice
                try process.run()
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                process.waitUntilExit()

                if process.terminationStatus == 0,
                    let prefix = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
                    !prefix.isEmpty
                {
                    prefixes.append(prefix)
                }
            }

            roots = prefixes
        }

        var checked: [String] = []
        for root in roots {
            for library in ["lib", "lib64"] {
                let path = "\(root)/\(library)/cmake/MayaFlux/MayaFluxConfigVersion.cmake"
                checked.append(path)
                guard FileManager.default.fileExists(atPath: path) else { continue }

                let source = try String(contentsOfFile: path, encoding: .utf8)
                let pattern = "set\\(PACKAGE_VERSION\\s+\"([^\"]+)\"\\)"
                let regex = try NSRegularExpression(pattern: pattern)
                let range = NSRange(source.startIndex..., in: source)
                guard let match = regex.firstMatch(in: source, range: range),
                    let capture = Range(match.range(at: 1), in: source)
                else {
                    throw CommunityUpdateError(message: "Malformed MayaFlux version file: \(path)")
                }

                let text = String(source[capture])
                return (text, try version(text, label: "installed MayaFlux"))
            }
        }

        throw CommunityUpdateError(
            message: "MayaFlux version file not found. Checked: \(checked.joined(separator: ", "))")
    }

    private static func registry() async throws -> [String: RegistryEntry] {
        let address = "https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json"
        guard let url = URL(string: address) else {
            throw CommunityUpdateError(message: "Invalid community registry URL")
        }

        let (data, response) = try await URLSession.shared.data(from: url)
        guard (response as? HTTPURLResponse)?.statusCode == 200,
            let records = try JSONSerialization.jsonObject(with: data) as? [[String: Any]]
        else {
            throw CommunityUpdateError(message: "Community registry must be a JSON array")
        }

        var result: [String: RegistryEntry] = [:]
        for record in records {
            guard let name = record["name"] as? String,
                let repo = record["repo"] as? String,
                let minimum = record["min_version"] as? String,
                name.range(of: "\\A[a-z][a-z0-9_]*\\z", options: .regularExpression) != nil,
                let repoURL = URL(string: repo), repoURL.scheme == "https", repoURL.host != nil
            else {
                throw CommunityUpdateError(message: "Community registry contains an invalid entry")
            }

            guard result[name] == nil else {
                throw CommunityUpdateError(message: "Duplicate registry entry for \(name)")
            }

            result[name] = RegistryEntry(
                repo: repo,
                minimum: try version(minimum, label: "registry entry for \(name)")
            )
        }
        return result
    }

    private static func clone(_ repo: String, to destination: String, report: (String) -> Void) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["clone", "--progress", "--depth=1", repo, destination]
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe

        try process.run()
        while true {
            let data = pipe.fileHandleForReading.availableData
            if data.isEmpty { break }
            report(String(decoding: data, as: UTF8.self).trimmingCharacters(in: .newlines))
        }
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            let name = (destination as NSString).lastPathComponent
            throw CommunityUpdateError(message: "Git clone failed for \(name)")
        }
    }

    static func run(projectPath: String, values: [String], report: (String) -> Void) async throws {
        let requested = try names(values)
        let project = (projectPath as NSString).expandingTildeInPath
        let manager = FileManager.default

        guard manager.fileExists(atPath: "\(project)/CMakeLists.txt") else {
            throw CommunityUpdateError(message: "Not a MayaFlux project: \(project)")
        }

        report("Fetching registry...")
        let entries = try await registry()
        let installed = try installedVersion()
        let community = "\(project)/community"
        var cloned: [String] = []

        do {
            for name in requested where entries[name] == nil {
                throw CommunityUpdateError(message: "Module \(name) is not in the community registry")
            }

            for name in requested {
                let entry = entries[name]!

                let module = "\(community)/\(name)"
                var isDirectory: ObjCBool = false
                let exists = manager.fileExists(atPath: module, isDirectory: &isDirectory)
                guard !exists || isDirectory.boolValue else {
                    throw CommunityUpdateError(message: "Community path for \(name) is not a directory: \(module)")
                }

                report("Preparing \(name)...")
                if !exists {
                    try manager.createDirectory(atPath: community, withIntermediateDirectories: true)
                    report("Cloning \(entry.repo) into \(module)")
                    defer {
                        if manager.fileExists(atPath: module) { cloned.append(name) }
                    }
                    try clone(entry.repo, to: module, report: report)
                } else {
                    report("Using existing community/\(name)")
                }

                guard manager.fileExists(atPath: "\(module)/\(name).cmake") else {
                    throw CommunityUpdateError(message: "Module \(name) is missing \(name).cmake")
                }
                guard manager.fileExists(atPath: "\(module)/src", isDirectory: &isDirectory),
                    isDirectory.boolValue
                else {
                    throw CommunityUpdateError(message: "Module \(name) is missing src/")
                }

                let manifestPath = "\(module)/community.json"
                guard manager.fileExists(atPath: manifestPath) else {
                    throw CommunityUpdateError(message: "Module \(name) is missing community.json")
                }

                let source = try String(contentsOfFile: manifestPath, encoding: .utf8)
                let manifest = try CommunityManifest.parse(source, expectedName: name)
                guard let requiredText = manifest["min_version"] as? String else {
                    throw CommunityUpdateError(message: "Module \(name) has no minimum version")
                }

                let required = max(entry.minimum, try version(requiredText, label: "manifest for \(name)"))
                guard installed.1 >= required else {
                    let minimum = required.parts.map(String.init).joined(separator: ".")
                    throw CommunityUpdateError(
                        message: "Module \(name) requires MayaFlux >= \(minimum); found \(installed.0)")
                }

                let packages = manifest["packages"] as? [String: Any] ?? [:]
                let manuals = manifest["manual_installs"] as? [Any] ?? []
                let assets = manifest["assets"] as? [Any] ?? []
                if !packages.isEmpty || !manuals.isEmpty || !assets.isEmpty {
                    report("\(name) has declared requirements pending review")
                }

                report("Validated \(name)")
            }

            let path = "\(project)/community.cmake"
            let current = manager.fileExists(atPath: path)
                ? try String(contentsOfFile: path, encoding: .utf8) : ""
            let existing = Set(current.split(separator: "\n").map {
                $0.trimmingCharacters(in: .whitespaces)
            })
            let additions = requested.filter { !existing.contains($0) }

            if !additions.isEmpty {
                let separator = current.isEmpty || current.hasSuffix("\n") ? "" : "\n"
                let updated = current + separator + additions.map { "\($0)\n" }.joined()
                try Data(updated.utf8).write(to: URL(fileURLWithPath: path), options: .atomic)
            }

            for name in requested {
                report(additions.contains(name)
                    ? "Registered \(name) in community.cmake"
                    : "\(name) was already registered")
            }
            report("Done. Rebuild the project to include the validated modules.")
        } catch {
            let remaining = cloned.isEmpty ? "none" : cloned.joined(separator: ", ")
            throw CommunityUpdateError(
                message: "\(error.localizedDescription). Clones left in community/: \(remaining)")
        }
    }
}
