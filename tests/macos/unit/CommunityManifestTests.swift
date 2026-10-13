import XCTest

final class CommunityManifestTests: XCTestCase {
    func testLegacyManifestReceivesAcquisitionDefaults() throws {
        let manifest = try CommunityManifest.parse(
            #"{"name":"terrain_field","min_version":"0.5.0"}"#,
            expectedName: "terrain_field")

        XCTAssertEqual(manifest["schema_version"] as? Int, 1)
        XCTAssertEqual(manifest["needs_lila"] as? Bool, false)
        XCTAssertEqual((manifest["packages"] as? [String: Any])?.count, 0)
        XCTAssertEqual((manifest["manual_installs"] as? [Any])?.count, 0)
        XCTAssertEqual((manifest["assets"] as? [Any])?.count, 0)
    }

    func testByteOrderMarkIsAccepted() throws {
        let manifest = try CommunityManifest.parse(
            "\u{FEFF}{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}")

        XCTAssertEqual(manifest["name"] as? String, "terrain_field")
    }

    func testDuplicateJSONKeysAreRejected() {
        XCTAssertThrowsError(try CommunityManifest.parse(
            #"{"name":"terrain_field","name":"other","min_version":"0.5.0"}"#)) { error in
            XCTAssertEqual((error as? ManifestError)?.path, "$")
            XCTAssertEqual((error as? ManifestError)?.reason, "Duplicate object key: name")
        }
    }

    func testRequestedNameMismatchReportsManifestField() {
        XCTAssertThrowsError(try CommunityManifest.parse(
            #"{"name":"terrain_field","min_version":"0.5.0"}"#,
            expectedName: "other")) { error in
            XCTAssertEqual((error as? ManifestError)?.path, "$.name")
        }
    }

    func testUnsafeHomebrewFormulaIsRejectedAtItsFieldPath() {
        let source = #"{"schema_version":1,"name":"terrain_field","min_version":"0.5.0","packages":{"macos":{"brew":{"formulae":["cmake; touch /tmp/weave-test"]}}}}"#

        XCTAssertThrowsError(try CommunityManifest.parse(source)) { error in
            XCTAssertEqual((error as? ManifestError)?.path, "$.packages.macos.brew.formulae[0]")
        }
    }

    func testTraversalInManualVerificationPathIsRejected() {
        let source = #"{"schema_version":1,"name":"terrain_field","min_version":"0.5.0","manual_installs":[{"id":"vendor_sdk","platforms":["macos"],"url":"https://vendor.example/sdk","description":"SDK","verify":{"path":"../bin/sdk"}}]}"#

        XCTAssertThrowsError(try CommunityManifest.parse(source)) { error in
            XCTAssertEqual((error as? ManifestError)?.path, "$.manual_installs[0].verify.path")
        }
    }

    func testAcquisitionDeclarationsRequireSupportedSchemaRevision() {
        let source = #"{"name":"terrain_field","min_version":"0.5.0","packages":{}}"#

        XCTAssertThrowsError(try CommunityManifest.parse(source)) { error in
            XCTAssertEqual((error as? ManifestError)?.path, "$.schema_version")
        }
    }
}
