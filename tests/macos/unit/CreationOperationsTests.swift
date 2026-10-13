import Foundation
import XCTest

final class CreationOperationsTests: XCTestCase {
    func testProjectArgumentsKeepUserValuesAsSeparateArguments() {
        XCTAssertEqual(
            ProjectCreation.arguments(
                name: "Demo; touch /tmp/unexpected",
                location: "/tmp/My Projects",
                withLila: true),
            ["new", "Demo; touch /tmp/unexpected", "/tmp/My Projects", "--with-lila"])
    }

    func testProjectArgumentsOmitDisabledOptionalFlag() {
        XCTAssertEqual(
            ProjectCreation.arguments(name: "Demo", location: "/tmp", withLila: false),
            ["new", "Demo", "/tmp"])
    }

    func testProjectArgumentsIncludeShaderCopyOptionWhenRequested() {
        XCTAssertEqual(
            ProjectCreation.arguments(
                name: "Demo", location: "/tmp/My Projects", withLila: false,
                copyShaderSources: true),
            ["new", "Demo", "/tmp/My Projects", "--copy-shader-sources"])
    }

    func testBundledCLIIsSelectedWhenInstalledCLIIsMissing() {
        let installed = URL(fileURLWithPath: "/home/user/.local/bin/weave")
        let bundled = URL(fileURLWithPath: "/Applications/Weave.app/Contents/Resources/weave")

        XCTAssertEqual(
            ProjectCreation.resolveExecutable(installed: installed, bundled: bundled) {
                $0 == bundled.path
            },
            bundled)
    }

    func testMissingCLIReturnsNoExecutable() {
        XCTAssertNil(
            ProjectCreation.resolveExecutable(
                installed: URL(fileURLWithPath: "/missing/installed"),
                bundled: URL(fileURLWithPath: "/missing/bundled")) { _ in false })
    }

    func testCommunityMetadataAppliesUserValuesAndDefaultsVersion() throws {
        let updated = try CommunityModuleCreation.metadata(
            source: #"{"name":"sample_module","min_version":"0.5.0","description":"Template","needs_lila":false}"#,
            expectedName: "sample_module",
            description: "A module",
            minVersion: "",
            needsLila: true)

        XCTAssertEqual(updated["name"] as? String, "sample_module")
        XCTAssertEqual(updated["min_version"] as? String, "0.5.0")
        XCTAssertEqual(updated["description"] as? String, "A module")
        XCTAssertEqual(updated["needs_lila"] as? Bool, true)
    }

    func testCommunityMetadataRejectsMalformedTemplate() {
        XCTAssertThrowsError(
            try CommunityModuleCreation.metadata(
                source: "{bad json",
                expectedName: "sample_module",
                description: "",
                minVersion: "0.5.0",
                needsLila: false))
    }

    func testApplyingMetadataUpdatesManifestAndCMakeSettings() throws {
        let module = try temporaryModule(
            manifest: #"{"name":"sample_module","min_version":"0.5.0","description":"Template","needs_lila":false}"#,
            cmake: "set(MF_MIN_VERSION \"0.5.0\")\nset(MF_NEEDS_LILA OFF)\n")
        defer { try? FileManager.default.removeItem(at: module.deletingLastPathComponent()) }

        try CommunityModuleCreation.apply(
            source: #"{"name":"sample_module","min_version":"0.5.0","description":"Template","needs_lila":false}"#,
            expectedName: "sample_module",
            description: "A module",
            minVersion: "0.6.1",
            needsLila: true,
            moduleDirectory: module)

        let manifest = try CommunityManifest.parse(
            String(contentsOf: module.appendingPathComponent("community.json"), encoding: .utf8),
            expectedName: "sample_module")
        let cmake = try String(
            contentsOf: module.appendingPathComponent("sample_module.cmake"), encoding: .utf8)
        XCTAssertEqual(manifest["description"] as? String, "A module")
        XCTAssertEqual(manifest["min_version"] as? String, "0.6.1")
        XCTAssertEqual(manifest["needs_lila"] as? Bool, true)
        XCTAssertTrue(cmake.contains("set(MF_MIN_VERSION \"0.6.1\")"))
        XCTAssertTrue(cmake.contains("set(MF_NEEDS_LILA ON)"))
    }

    func testUnexpectedCMakeTemplateLeavesManifestUnchanged() throws {
        let original = #"{"name":"sample_module","min_version":"0.5.0","description":"Template","needs_lila":false}"#
        let module = try temporaryModule(manifest: original, cmake: "project(sample_module)\n")
        defer { try? FileManager.default.removeItem(at: module.deletingLastPathComponent()) }

        XCTAssertThrowsError(
            try CommunityModuleCreation.apply(
                source: original,
                expectedName: "sample_module",
                description: "A module",
                minVersion: "0.6.1",
                needsLila: true,
                moduleDirectory: module))
        XCTAssertEqual(
            try String(contentsOf: module.appendingPathComponent("community.json"), encoding: .utf8),
            original)
    }

    private func temporaryModule(manifest: String, cmake: String) throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        let module = root.appendingPathComponent("sample_module", isDirectory: true)
        try FileManager.default.createDirectory(at: module, withIntermediateDirectories: true)
        try Data(manifest.utf8).write(to: module.appendingPathComponent("community.json"))
        try Data(cmake.utf8).write(to: module.appendingPathComponent("sample_module.cmake"))
        return module
    }
}
