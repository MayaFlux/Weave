import Foundation

enum InstallStep {
    case channelSelect
    case passwordPrompt
    case running
    case done
    case failed(String)
}

enum HomebrewLocator {
    static func discover(
        path: String, optDirectories: [String], isExecutable: (String) -> Bool
    ) -> String? {
        let pathDirectories = path.split(separator: ":").map(String.init)
        let optBinDirectories = optDirectories.map { "/opt/\($0)/bin" }
        return (pathDirectories + optBinDirectories)
            .filter { !$0.hasPrefix("/usr/local") }
            .map { "\($0)/brew" }
            .first(where: isExecutable)
    }
}

enum InstallerWorkflow {
    static func run(
        formula: String,
        findBrew: () -> String?,
        runCommand: (String) async -> Int32,
        configureEnvironment: (String, String) async -> String?,
        log: (String) -> Void
    ) async -> InstallStep {
        guard ["mayaflux", "mayaflux-dev"].contains(formula) else {
            return .failed("Unsupported MayaFlux formula: \(formula)")
        }

        let brew: String
        if let found = findBrew() {
            brew = found
            log("✅ Homebrew found at \(brew)")
        } else {
            log("➤ Installing Homebrew...")
            log("  This may take a few minutes...")
            let code = await runCommand(
                "NONINTERACTIVE=1 /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"")
            guard code == 0 else {
                return .failed(
                    "Homebrew installation failed.\n\nCheck your internet connection and try again.")
            }
            guard let found = findBrew() else {
                return .failed(
                    "Homebrew installed but executable not found.\nCheck your PATH or Homebrew installation under /opt.")
            }
            brew = found
            log("✅ Homebrew installed at \(brew)")
        }

        let escapedBrew = shellEscape(brew)
        let conflicting = formula == "mayaflux" ? "mayaflux-dev" : "mayaflux"
        let conflictCode = await runCommand(
            "\(escapedBrew) list --formula | grep -E '^\(conflicting)$'")
        if conflictCode == 0 {
            log("➤ Removing \(conflicting)...")
            let removeCode = await runCommand("\(escapedBrew) uninstall \(conflicting)")
            guard removeCode == 0 else { return .failed("Failed to remove \(conflicting).") }
            log("✅ \(conflicting) removed")
        }

        log("➤ Adding tap mayaflux/mayaflux...")
        let tapCode = await runCommand("\(escapedBrew) tap mayaflux/mayaflux")
        guard tapCode == 0 else {
            return .failed("Failed to add mayaflux tap.\n\nCheck your internet connection.")
        }
        log("✅ Tap added")

        let trustAvailable = await runCommand("\(escapedBrew) help trust >/dev/null 2>&1") == 0
        if trustAvailable {
            log("➤ Trusting the mayaflux/mayaflux tap...")
            let trustCode = await runCommand(
                "\(escapedBrew) trust --tap mayaflux/mayaflux")
            guard trustCode == 0 else {
                return .failed(
                    "Failed to trust the mayaflux/mayaflux tap.\n\n"
                        + "Review the log, then run: \(brew) trust --tap mayaflux/mayaflux")
            }
            log("✅ Tap trusted")
        } else {
            log("  This Homebrew version does not provide tap trust.")
        }

        log("➤ Installing \(formula)...")
        log("  ⏳ This may take several minutes...")
        let installCode = await runCommand("\(escapedBrew) install \(formula)")
        guard installCode == 0 else {
            return .failed("brew install \(formula) failed.\n\nSee log above for details.")
        }
        log("✅ \(formula) installed")

        log("➤ Verifying installation...")
        let verifyCode = await runCommand(
            "\(escapedBrew) list --formula | grep -E '^\(formula)$'")
        guard verifyCode == 0 else {
            return .failed("Installation verification failed — \(formula) not found in brew list.")
        }
        log("✅ Verified")

        if let error = await configureEnvironment(brew, formula) {
            return .failed(error)
        }
        return .done
    }

    private static func shellEscape(_ value: String) -> String {
        "'" + value.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }
}
