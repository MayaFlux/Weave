import XCTest

final class HomebrewLocatorTests: XCTestCase {
    func testPathEntriesTakePrecedenceOverOptDiscovery() {
        var checked: [String] = []
        let result = HomebrewLocator.discover(
            path: "/custom/tools:/other/bin", optDirectories: ["homebrew"],
            isExecutable: { candidate in
                checked.append(candidate)
                return candidate == "/custom/tools/brew" || candidate == "/opt/homebrew/bin/brew"
            })

        XCTAssertEqual(result, "/custom/tools/brew")
        XCTAssertEqual(checked, ["/custom/tools/brew"])
    }

    func testOptDiscoveryFindsHomebrewWithoutHardcodedPrefix() {
        let result = HomebrewLocator.discover(
            path: "/usr/bin:/bin", optDirectories: ["homebrew"],
            isExecutable: { $0 == "/opt/homebrew/bin/brew" })

        XCTAssertEqual(result, "/opt/homebrew/bin/brew")
    }

    func testIntelHomebrewLocationIsNotUsedAsFallback() {
        var checked: [String] = []
        let result = HomebrewLocator.discover(
            path: "/usr/local/bin:/usr/bin", optDirectories: [],
            isExecutable: { checked.append($0); return true })

        XCTAssertEqual(result, "/usr/bin/brew")
        XCTAssertEqual(checked, ["/usr/bin/brew"])
    }

    func testMissingExecutableReturnsNoHomebrew() {
        XCTAssertNil(HomebrewLocator.discover(
            path: "/usr/bin", optDirectories: ["homebrew"], isExecutable: { _ in false }))
    }
}
