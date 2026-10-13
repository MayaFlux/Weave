import XCTest

final class CommunityUpdateTests: XCTestCase {
    func testNamesPreserveOrderAndRemoveDuplicates() throws {
        XCTAssertEqual(
            try CommunityUpdate.names([" terrain_field, noise ", "noise,sky"]),
            ["terrain_field", "noise", "sky"])
    }

    func testEmptyNameListIsRejected() {
        XCTAssertThrowsError(try CommunityUpdate.names([])) { error in
            XCTAssertEqual((error as? CommunityUpdateError)?.message,
                "At least one module name is required")
        }
    }

    func testEmptyCommaSeparatedNameIsRejected() {
        XCTAssertThrowsError(try CommunityUpdate.names(["terrain_field,"])) { error in
            XCTAssertEqual((error as? CommunityUpdateError)?.message,
                "Empty module name in request")
        }
    }

    func testUnsafeModuleNameIsRejected() {
        XCTAssertThrowsError(try CommunityUpdate.names(["../terrain"])) { error in
            XCTAssertEqual((error as? CommunityUpdateError)?.message,
                "Invalid module name: ../terrain")
        }
    }

    func testInvalidProjectIsRejectedBeforeRegistryAccess() async {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        defer { try? FileManager.default.removeItem(at: root) }
        var registryRequested = false
        let dependencies = updateDependencies(registry: {
            registryRequested = true
            return [:]
        })

        do {
            try await CommunityUpdate.run(
                projectPath: root.path, values: ["terrain_field"], report: { _ in },
                dependencies: dependencies)
            XCTFail("An invalid project should be rejected")
        } catch {
            XCTAssertTrue(error.localizedDescription.contains("Not a MayaFlux project"))
        }

        XCTAssertFalse(registryRequested)
        XCTAssertFalse(FileManager.default.fileExists(
            atPath: root.appendingPathComponent("community.cmake").path))
    }

    func testValidatedModuleIsRegisteredOnceAcrossRepeatedUpdates() async throws {
        let project = try makeProject()
        defer { try? FileManager.default.removeItem(at: project) }
        let dependencies = updateDependencies(clone: { _, destination in
            try self.makeModule(named: "terrain_field", at: URL(fileURLWithPath: destination))
        })
        var reports: [String] = []

        try await CommunityUpdate.run(
            projectPath: project.path, values: ["terrain_field"], report: { reports.append($0) },
            dependencies: dependencies)
        try await CommunityUpdate.run(
            projectPath: project.path, values: ["terrain_field"], report: { reports.append($0) },
            dependencies: dependencies)

        XCTAssertEqual(
            try String(contentsOf: project.appendingPathComponent("community.cmake"), encoding: .utf8),
            "terrain_field\n")
        XCTAssertTrue(reports.contains("terrain_field was already registered"))
    }

    func testFailedPartialCloneIsReportedAndNotRegistered() async throws {
        let project = try makeProject()
        defer { try? FileManager.default.removeItem(at: project) }
        let dependencies = updateDependencies(clone: { _, destination in
            let partial = URL(fileURLWithPath: destination)
            try FileManager.default.createDirectory(at: partial, withIntermediateDirectories: true)
            try Data("partial".utf8).write(to: partial.appendingPathComponent("README"))
            throw CommunityUpdateError(message: "Clone failed")
        })

        do {
            try await CommunityUpdate.run(
                projectPath: project.path, values: ["terrain_field"], report: { _ in },
                dependencies: dependencies)
            XCTFail("A failed clone should fail the update")
        } catch {
            XCTAssertTrue(error.localizedDescription.contains("Clones left in community/: terrain_field"))
        }

        XCTAssertTrue(FileManager.default.fileExists(
            atPath: project.appendingPathComponent("community/terrain_field/README").path))
        XCTAssertFalse(FileManager.default.fileExists(
            atPath: project.appendingPathComponent("community.cmake").path))
    }

    func testMalformedClonedManifestLeavesRegistrationUnchanged() async throws {
        let project = try makeProject()
        defer { try? FileManager.default.removeItem(at: project) }
        let dependencies = updateDependencies(clone: { _, destination in
            let module = URL(fileURLWithPath: destination)
            try FileManager.default.createDirectory(
                at: module.appendingPathComponent("src"), withIntermediateDirectories: true)
            try Data("set(MF_MIN_VERSION \"0.5.0\")\n".utf8)
                .write(to: module.appendingPathComponent("terrain_field.cmake"))
            try Data("{bad json".utf8).write(to: module.appendingPathComponent("community.json"))
        })

        do {
            try await CommunityUpdate.run(
                projectPath: project.path, values: ["terrain_field"], report: { _ in },
                dependencies: dependencies)
            XCTFail("A malformed manifest should fail the update")
        } catch {
            XCTAssertTrue(error.localizedDescription.contains("Invalid JSON"))
        }

        XCTAssertFalse(FileManager.default.fileExists(
            atPath: project.appendingPathComponent("community.cmake").path))
    }

    private func makeProject() throws -> URL {
        let project = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: project, withIntermediateDirectories: true)
        try Data("project(UnitFixture)\n".utf8)
            .write(to: project.appendingPathComponent("CMakeLists.txt"))
        return project
    }

    private func makeModule(named name: String, at destination: URL) throws {
        try FileManager.default.createDirectory(
            at: destination.appendingPathComponent("src"), withIntermediateDirectories: true)
        try Data("set(MF_MIN_VERSION \"0.5.0\")\n".utf8)
            .write(to: destination.appendingPathComponent("\(name).cmake"))
        let manifest = """
            {"name":"\(name)","min_version":"0.5.0"}
            """
        try Data(manifest.utf8).write(to: destination.appendingPathComponent("community.json"))
    }

    private func updateDependencies(
        registry: @escaping () async throws -> [String: CommunityUpdate.RegistryEntry] = {
            ["terrain_field": .init(
                repo: "https://example.invalid/terrain_field.git",
                minimum: .init(parts: [0, 5, 0]))]
        },
        clone: @escaping (String, String) throws -> Void = { _, _ in
            throw CommunityUpdateError(message: "Unexpected clone")
        }
    ) -> CommunityUpdate.Dependencies {
        CommunityUpdate.Dependencies(
            registry: registry,
            installedVersion: { ("0.8.0", .init(parts: [0, 8, 0])) },
            clone: { repo, destination, _ in try clone(repo, destination) })
    }
}
