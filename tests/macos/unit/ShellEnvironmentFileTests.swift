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
