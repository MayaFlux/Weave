import Foundation

enum ShellEnvironmentFile {
    static func resolvePath(
        home: URL, zdotdir: String?, fileManager: FileManager = .default
    ) -> URL {
        if let zdotdir, !zdotdir.isEmpty {
            var isDirectory: ObjCBool = false
            if fileManager.fileExists(atPath: zdotdir, isDirectory: &isDirectory),
                isDirectory.boolValue
            {
                return URL(fileURLWithPath: zdotdir).appendingPathComponent(".zshenv")
            }
        }
        return home.appendingPathComponent(".zshenv")
    }

    @discardableResult
    static func update(
        at path: URL, brewPrefix: String, fileManager: FileManager = .default
    ) throws -> Bool {
        let exists = fileManager.fileExists(atPath: path.path)
        let original = exists
            ? try String(contentsOf: path, encoding: .utf8)
            : ""

        if exists && !original.contains("# MayaFlux") {
            let backup = path.appendingPathExtension("weave-backup")
            try Data(original.utf8).write(to: backup, options: .atomic)
        }

        var inManagedBlock = false
        let retainedLines = original.components(separatedBy: .newlines).filter { line in
            if line.contains("# MayaFlux") {
                inManagedBlock = true
                return false
            }
            if inManagedBlock {
                let isManagedSource = line.trimmingCharacters(in: .whitespaces)
                    .hasPrefix("source ")
                if isManagedSource { return false }
                inManagedBlock = false
            }
            let legacySource = line.range(
                of: "source.*mayaflux.*/env\\.sh", options: .regularExpression) != nil
            return !line.contains("MAYAFLUX_ROOT")
                && !line.contains("mayaflux_env.sh")
                && !legacySource
                && !(line.contains(".local/bin") && line.contains("$PATH"))
        }
        var retained = retainedLines.joined(separator: "\n")
        while retained.hasSuffix("\n") { retained.removeLast() }

        let source = shellQuote(URL(fileURLWithPath: brewPrefix).appendingPathComponent("env.sh").path)
        let block = "# MayaFlux (installed via Homebrew)\nsource \(source)\n"
            + "export PATH=\"$HOME/.local/bin:$PATH\"\n"
        let separator = retained.isEmpty ? "" : "\n\n"
        try Data((retained + separator + block).utf8).write(to: path, options: .atomic)
        return !exists
    }

    private static func shellQuote(_ value: String) -> String {
        "'" + value.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }
}
