import Foundation
import XCTest

final class ShellEnvironmentFileTests: XCTestCase {
    func testConfiguredZDOTDIRSelectsItsZshenv() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        let custom = root.appendingPathComponent("zsh", isDirectory: true)
        try FileManager.default.createDirectory(at: custom, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }

        let selected = ShellEnvironmentFile.resolvePath(home: root, zdotdir: custom.path)

        XCTAssertEqual(selected, custom.appendingPathComponent(".zshenv"))
    }

    func testConfiguredZDOTDIRTakesPrecedenceOverHomeZshenvDuringUpdate() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        let custom = root.appendingPathComponent("custom zsh", isDirectory: true)
        try FileManager.default.createDirectory(at: custom, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let homeFile = root.appendingPathComponent(".zshenv")
        let customFile = custom.appendingPathComponent(".zshenv")
        try Data("export HOME_CONFIG=preserve\n".utf8).write(to: homeFile)
        try Data("export CUSTOM_CONFIG=preserve\n".utf8).write(to: customFile)

        let selected = ShellEnvironmentFile.resolvePath(home: root, zdotdir: custom.path)
        _ = try ShellEnvironmentFile.update(at: selected, brewPrefix: "/opt/Home brew")

        XCTAssertEqual(try String(contentsOf: homeFile, encoding: .utf8), "export HOME_CONFIG=preserve\n")
        let customContents = try String(contentsOf: customFile, encoding: .utf8)
        XCTAssertTrue(customContents.hasPrefix("export CUSTOM_CONFIG=preserve\n"))
        XCTAssertTrue(customContents.contains("source '/opt/Home brew/env.sh'"))
    }

    func testUnsetZDOTDIRUsesExistingHomeZshenv() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let expected = root.appendingPathComponent(".zshenv")
        try Data("export USER_VALUE=yes\n".utf8).write(to: expected)

        let selected = ShellEnvironmentFile.resolvePath(home: root, zdotdir: nil)

        XCTAssertEqual(selected, expected)
    }

    func testUnsetZDOTDIRCreatesHomeZshenvWhenMissing() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let expected = root.appendingPathComponent(".zshenv")

        let selected = ShellEnvironmentFile.resolvePath(home: root, zdotdir: nil)
        let created = try ShellEnvironmentFile.update(at: selected, brewPrefix: "/opt/homebrew")

        XCTAssertEqual(selected, expected)
        XCTAssertTrue(created)
        XCTAssertTrue(FileManager.default.fileExists(atPath: expected.path))
    }

    func testMissingZDOTDIRDirectoryFallsBackToHomeZshenv() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }

        let selected = ShellEnvironmentFile.resolvePath(
            home: root, zdotdir: root.appendingPathComponent("missing").path)

        XCTAssertEqual(selected, root.appendingPathComponent(".zshenv"))
    }

    func testZDOTDIRThatNamesAFileFallsBackToHomeZshenv() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let notDirectory = root.appendingPathComponent("not-a-directory")
        try Data("file\n".utf8).write(to: notDirectory)

        XCTAssertEqual(
            ShellEnvironmentFile.resolvePath(home: root, zdotdir: notDirectory.path),
            root.appendingPathComponent(".zshenv"))
    }

    func testUpdateAppendsMayaFluxConfigurationAfterExistingShellSetup() throws {
        let file = try temporaryFile(contents: "eval \"$(brew shellenv)\"\nexport OTHER=value\n")
        defer { try? FileManager.default.removeItem(at: file.deletingLastPathComponent()) }

        let created = try ShellEnvironmentFile.update(at: file, brewPrefix: "/opt/homebrew")
        let contents = try String(contentsOf: file, encoding: .utf8)

        XCTAssertFalse(created)
        XCTAssertTrue(contents.hasPrefix("eval \"$(brew shellenv)\"\nexport OTHER=value\n"))
        XCTAssertTrue(contents.hasSuffix(
            "# MayaFlux (installed via Homebrew)\nsource '/opt/homebrew/env.sh'\n"
                + "export PATH=\"$HOME/.local/bin:$PATH\"\n"))
    }

    func testUpdatingExistingManagedBlockReplacesItWithoutDuplication() throws {
        let old = "# MayaFlux (installed via Homebrew)\nsource '/opt/old/env.sh'\n"
        let file = try temporaryFile(contents: "export USER_VALUE=yes\n\(old)")
        defer { try? FileManager.default.removeItem(at: file.deletingLastPathComponent()) }

        _ = try ShellEnvironmentFile.update(at: file, brewPrefix: "/opt/current")
        let contents = try String(contentsOf: file, encoding: .utf8)

        XCTAssertEqual(contents.components(separatedBy: "# MayaFlux").count - 1, 1)
        XCTAssertFalse(contents.contains("/opt/old/env.sh"))
        XCTAssertTrue(contents.contains("source '/opt/current/env.sh'"))
        XCTAssertTrue(contents.hasPrefix("export USER_VALUE=yes\n"))
    }

    func testUpdatingLegacyMayaFluxAndWeavePathEntriesLeavesUserLinesAndSingleManagedBlock() throws {
        let original = "export USER_VALUE=yes\nexport MAYAFLUX_ROOT=/old/prefix\n"
            + "source /old/mayaflux/share/MayaFlux/env.sh\n"
            + "export PATH=\"$HOME/.local/bin:$PATH\"\n"
            + "export PATH=\"$PATH:/custom/bin\"\n"
        let file = try temporaryFile(contents: original)
        defer { try? FileManager.default.removeItem(at: file.deletingLastPathComponent()) }

        _ = try ShellEnvironmentFile.update(at: file, brewPrefix: "/opt/homebrew")
        let contents = try String(contentsOf: file, encoding: .utf8)

        XCTAssertTrue(contents.hasPrefix("export USER_VALUE=yes\n"))
        XCTAssertTrue(contents.contains("export PATH=\"$PATH:/custom/bin\""))
        XCTAssertFalse(contents.contains("/old/prefix"))
        XCTAssertEqual(contents.components(separatedBy: "# MayaFlux").count - 1, 1)
        XCTAssertEqual(contents.components(separatedBy: "source '/opt/homebrew/env.sh'").count - 1, 1)
    }

    func testMalformedUTF8IsReportedWithoutReplacingEnvironmentFile() throws {
        let folder = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let file = folder.appendingPathComponent(".zshenv")
        let original = Data([0x65, 0x78, 0x70, 0x6f, 0x72, 0x74, 0xff])
        try original.write(to: file)

        XCTAssertThrowsError(try ShellEnvironmentFile.update(at: file, brewPrefix: "/opt/homebrew"))
        XCTAssertEqual(try Data(contentsOf: file), original)
    }

    func testOriginalUserConfigurationIsBackedUpBeforeUpdate() throws {
        let original = "export USER_VALUE=yes\n"
        let file = try temporaryFile(contents: original)
        defer { try? FileManager.default.removeItem(at: file.deletingLastPathComponent()) }

        _ = try ShellEnvironmentFile.update(at: file, brewPrefix: "/opt/homebrew")

        XCTAssertEqual(
            try String(contentsOf: file.appendingPathExtension("weave-backup"), encoding: .utf8),
            original)
    }

    func testMissingEnvironmentFileIsCreated() throws {
        let folder = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let file = folder.appendingPathComponent(".zshenv")

        let created = try ShellEnvironmentFile.update(at: file, brewPrefix: "/opt/homebrew")

        XCTAssertTrue(created)
        XCTAssertTrue(FileManager.default.fileExists(atPath: file.path))
        XCTAssertTrue(try String(contentsOf: file, encoding: .utf8)
            .contains("source '/opt/homebrew/env.sh'"))
    }

    private func temporaryFile(contents: String) throws -> URL {
        let folder = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let file = folder.appendingPathComponent(".zshenv")
        try Data(contents.utf8).write(to: file)
        return file
    }
}
