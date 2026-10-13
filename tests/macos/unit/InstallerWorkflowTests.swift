import XCTest

final class InstallerWorkflowTests: XCTestCase {
    func testMissingHomebrewStopsAfterFailedBootstrapAndReportsRecovery() async {
        var commands: [String] = []
        var configurationRequested = false
        let result = await InstallerWorkflow.run(
            formula: "mayaflux", findBrew: { nil },
            runCommand: { command in
                commands.append(command)
                return 1
            },
            configureEnvironment: { _, _ in
                configurationRequested = true
                return nil
            }, log: { _ in })

        guard case .failed(let message) = result else {
            return XCTFail("Failed Homebrew bootstrap should fail the workflow")
        }
        XCTAssertTrue(message.contains("Homebrew installation failed"))
        XCTAssertEqual(commands.count, 1)
        XCTAssertFalse(configurationRequested)
    }

    func testTapFailureStopsBeforeTrustAndFormulaInstall() async {
        var commands: [String] = []
        let brew = "/opt/homebrew/bin/brew"
        let result = await InstallerWorkflow.run(
            formula: "mayaflux", findBrew: { brew },
            runCommand: { command in
                commands.append(command)
                return command.contains("list --formula") ? 1 : 2
            },
            configureEnvironment: { _, _ in nil }, log: { _ in })

        guard case .failed(let message) = result else {
            return XCTFail("A failed tap should fail the workflow")
        }
        XCTAssertTrue(message.contains("Failed to add mayaflux tap"))
        XCTAssertEqual(commands, [
            "'/opt/homebrew/bin/brew' list --formula | grep -E '^mayaflux-dev$'",
            "'/opt/homebrew/bin/brew' tap mayaflux/mayaflux",
        ])
    }

    func testFailedConflictRemovalStopsBeforeAddingTap() async {
        var commands: [String] = []
        let result = await InstallerWorkflow.run(
            formula: "mayaflux", findBrew: { "/opt/homebrew/bin/brew" },
            runCommand: { command in
                commands.append(command)
                return command.contains("list --formula") ? 0 : 1
            },
            configureEnvironment: { _, _ in nil }, log: { _ in })

        guard case .failed(let message) = result else {
            return XCTFail("A failed conflicting formula removal should fail")
        }
        XCTAssertTrue(message.contains("Failed to remove mayaflux-dev"))
        XCTAssertEqual(commands.count, 2)
        XCTAssertTrue(commands[1].contains("uninstall mayaflux-dev"))
    }

    func testSuccessfulInstallWithoutHomebrewTrustConfiguresEnvironment() async {
        var commands: [String] = []
        var configured: (String, String)?
        var logs: [String] = []
        let brew = "/opt/Home brew/bin/brew"
        let result = await InstallerWorkflow.run(
            formula: "mayaflux", findBrew: { brew },
            runCommand: { command in
                commands.append(command)
                if command.contains("help trust") { return 1 }
                if command.contains("^mayaflux-dev$") { return 1 }
                return 0
            },
            configureEnvironment: { path, formula in
                configured = (path, formula)
                return nil
            }, log: { logs.append($0) })

        guard case .done = result else {
            return XCTFail("Successful install and configuration should finish")
        }
        XCTAssertEqual(configured?.0, brew)
        XCTAssertEqual(configured?.1, "mayaflux")
        XCTAssertTrue(commands.contains("'/opt/Home brew/bin/brew' install mayaflux"))
        XCTAssertTrue(logs.contains("  This Homebrew version does not provide tap trust."))
    }

    func testTrustFailureStopsBeforeFormulaInstall() async {
        var commands: [String] = []
        let result = await InstallerWorkflow.run(
            formula: "mayaflux", findBrew: { "/opt/homebrew/bin/brew" },
            runCommand: { command in
                commands.append(command)
                if command.contains("help trust") { return 0 }
                if command.contains("trust --tap") { return 1 }
                return 0
            },
            configureEnvironment: { _, _ in nil }, log: { _ in })

        guard case .failed(let message) = result else {
            return XCTFail("Failed tap trust should fail the workflow")
        }
        XCTAssertTrue(message.contains("Failed to trust the mayaflux/mayaflux tap"))
        XCTAssertFalse(commands.contains(where: { $0.contains(" install mayaflux") }))
    }

    func testTapIsTrustedBeforeFormulaInstallWhenTrustIsAvailable() async {
        var commands: [String] = []
        let result = await InstallerWorkflow.run(
            formula: "mayaflux", findBrew: { "/opt/homebrew/bin/brew" },
            runCommand: { command in
                commands.append(command)
                if command.contains("^mayaflux-dev$") { return 1 }
                return 0
            },
            configureEnvironment: { _, _ in nil }, log: { _ in })

        guard case .done = result else {
            return XCTFail("A trusted tap should allow installation to complete")
        }
        let tap = try? XCTUnwrap(commands.firstIndex(of: "'/opt/homebrew/bin/brew' tap mayaflux/mayaflux"))
        let trust = try? XCTUnwrap(commands.firstIndex(of: "'/opt/homebrew/bin/brew' trust --tap mayaflux/mayaflux"))
        let install = try? XCTUnwrap(commands.firstIndex(of: "'/opt/homebrew/bin/brew' install mayaflux"))
        XCTAssertNotNil(tap)
        XCTAssertNotNil(trust)
        XCTAssertNotNil(install)
        if let tap, let trust, let install {
            XCTAssertLessThan(tap, trust)
            XCTAssertLessThan(trust, install)
        }
    }

    func testFormulaInstallFailureDoesNotConfigureEnvironment() async {
        var configurationRequested = false
        let result = await InstallerWorkflow.run(
            formula: "mayaflux-dev", findBrew: { "/opt/homebrew/bin/brew" },
            runCommand: { command in
                if command.contains("^mayaflux$") { return 1 }
                if command.contains(" install ") { return 1 }
                return 0
            },
            configureEnvironment: { _, _ in
                configurationRequested = true
                return nil
            }, log: { _ in })

        guard case .failed(let message) = result else {
            return XCTFail("A failed formula install should fail the workflow")
        }
        XCTAssertTrue(message.contains("brew install mayaflux-dev failed"))
        XCTAssertFalse(configurationRequested)
    }

    func testConfigurationFailureIsReturnedToInstallerState() async {
        let result = await InstallerWorkflow.run(
            formula: "mayaflux", findBrew: { "/opt/homebrew/bin/brew" },
            runCommand: { command in
                if command.contains("help trust") || command.contains("^mayaflux-dev$") { return 1 }
                return 0
            },
            configureEnvironment: { _, _ in "Could not determine Homebrew prefix." },
            log: { _ in })

        guard case .failed(let message) = result else {
            return XCTFail("Configuration errors should be visible as a failed workflow")
        }
        XCTAssertEqual(message, "Could not determine Homebrew prefix.")
    }

    func testUnsupportedFormulaCannotReachShellCommands() async {
        var commands: [String] = []
        let result = await InstallerWorkflow.run(
            formula: "mayaflux; touch /tmp/weave-test", findBrew: { "/opt/homebrew/bin/brew" },
            runCommand: { command in commands.append(command); return 0 },
            configureEnvironment: { _, _ in nil }, log: { _ in })

        guard case .failed(let message) = result else {
            return XCTFail("Unsupported formula should be rejected")
        }
        XCTAssertTrue(message.contains("Unsupported MayaFlux formula"))
        XCTAssertTrue(commands.isEmpty)
    }

    func testInstalledFormulaMissingDuringVerificationDoesNotConfigureEnvironment() async {
        var configurationRequested = false
        let result = await InstallerWorkflow.run(
            formula: "mayaflux", findBrew: { "/opt/homebrew/bin/brew" },
            runCommand: { command in
                if command.contains("help trust") || command.contains("^mayaflux-dev$") { return 1 }
                if command.contains("^mayaflux$") { return 1 }
                return 0
            },
            configureEnvironment: { _, _ in
                configurationRequested = true
                return nil
            }, log: { _ in })

        guard case .failed(let message) = result else {
            return XCTFail("A missing installed formula should fail verification")
        }
        XCTAssertTrue(message.contains("Installation verification failed"))
        XCTAssertFalse(configurationRequested)
    }
}
