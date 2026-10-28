import XCTest

final class CommunityManifestVectorTests: XCTestCase {
    private static var vectors: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("vectors/community")
    }

    private func files() throws -> [URL] {
        try FileManager.default.contentsOfDirectory(
            at: Self.vectors, includingPropertiesForKeys: nil
        ).filter { $0.pathExtension == "json" }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
    }

    func testCorpusIsPresent() throws {
        let names = try files().map(\.lastPathComponent)

        XCTAssertTrue(names.contains { $0.hasPrefix("valid-") }, Self.vectors.path)
        XCTAssertTrue(names.contains { $0.hasPrefix("invalid-") }, Self.vectors.path)
        for name in names {
            XCTAssertTrue(name.hasPrefix("valid-") || name.hasPrefix("invalid-"), name)
        }
    }

    func testEveryVectorIsAcceptedOrRejectedAsNamed() throws {
        for file in try files() {
            let source = try String(contentsOf: file, encoding: .utf8)
            let object = try JSONSerialization.jsonObject(with: Data(source.utf8)) as? [String: Any]
            let name = try XCTUnwrap(object?["name"] as? String, file.lastPathComponent)

            if file.lastPathComponent.hasPrefix("valid-") {
                XCTAssertNoThrow(
                    try CommunityManifest.parse(source, expectedName: name), file.lastPathComponent)
            } else {
                XCTAssertThrowsError(
                    try CommunityManifest.parse(source, expectedName: name), file.lastPathComponent
                ) { error in
                    XCTAssertTrue(error is ManifestError, file.lastPathComponent)
                }
            }
        }
    }
}
