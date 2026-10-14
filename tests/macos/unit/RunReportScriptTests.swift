import Foundation
import XCTest

final class RunReportScriptTests: XCTestCase {
    private var root: URL!
    private var project: URL!
    private var install: URL!
    private var config: URL!

    private struct Result {
        let status: Int32
        let output: String
    }

    override func setUpWithError() throws {
        let files = FileManager.default
        let created = try files.url(
            for: .itemReplacementDirectory, in: .userDomainMask,
            appropriateFor: files.temporaryDirectory, create: true)
        let physical = try XCTUnwrap(realpath(created.path, nil))
        root = URL(fileURLWithPath: String(cString: physical))
        free(physical)
        project = root.appendingPathComponent("User's Project [test] & spaces")
        install = root.appendingPathComponent("Maya Flux [install]")
        config = install.appendingPathComponent("share/MayaFlux/report/mayaflux.report.json")
        try files.createDirectory(
            at: config.deletingLastPathComponent(), withIntermediateDirectories: true)
        try files.createDirectory(
            at: project.appendingPathComponent("build"), withIntermediateDirectories: true)
        try "{\"journal\":{}}\n".write(to: config, atomically: true, encoding: .utf8)
        let script = try String(contentsOf: Self.template, encoding: .utf8)
            .replacingOccurrences(of: "@PROJECT_NAME@", with: "ReportFixture")
        try writeExecutable(script: script, named: "run-report.sh", in: project)
        try installApplication(at: "build/ReportFixture")
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: root)
    }

    func testRunsApplicationWithConfigurationAndCapturesBothChannels() throws {
        let result = try runScript(environment: ["WEAVE_REPORT_VALUE": "terminal value"])
        let capture = try read("mayaflux_run.txt")

        XCTAssertEqual(result.status, 0, result.output)
        XCTAssertTrue(capture.contains("cwd=\(project.path)"))
        XCTAssertTrue(capture.contains("config=\(config.path)"))
        XCTAssertTrue(capture.contains("environment=terminal value"))
        XCTAssertTrue(capture.contains("stdout: diagnostic output"))
        XCTAssertTrue(capture.contains("stderr: crash details"))
        XCTAssertEqual(try read("data/log/mayaflux_report.log"), "fresh application report")
        XCTAssertTrue(result.output.contains(project.appendingPathComponent("mayaflux_run.txt").path))
    }

    func testApplicationFailureIsReturnedAndPreviousLogsArePreserved() throws {
        try write("previous console", to: "mayaflux_run.txt")
        try write("previous report", to: "data/log/mayaflux_report.log")

        let result = try runScript(environment: ["WEAVE_REPORT_EXIT": "37"])

        XCTAssertEqual(result.status, 37, result.output)
        XCTAssertTrue(result.output.contains("Application exit code: 37"))
        XCTAssertEqual(try previous("mayaflux_run.txt"), ["previous console"])
        XCTAssertEqual(try previous("data/log/mayaflux_report.log"), ["previous report"])
        XCTAssertEqual(try read("data/log/mayaflux_report.log"), "fresh application report")
    }

    func testEarlyCrashExplainsMissingApplicationReport() throws {
        let result = try runScript(environment: ["WEAVE_REPORT_EXIT": "23", "WEAVE_REPORT_NO_LOG": "1"])

        XCTAssertEqual(result.status, 23, result.output)
        XCTAssertTrue(result.output.contains("Application report was not created"))
        XCTAssertTrue(try read("mayaflux_run.txt").contains("stderr: crash details"))
    }

    func testUnresolvedProjectNameFallsBackToCMakeCache() throws {
        let script = try String(contentsOf: Self.template, encoding: .utf8)
        try writeExecutable(script: script, named: "run-report.sh", in: project)
        try write("CMAKE_PROJECT_NAME:STATIC=ReportFixture\n", to: "build/CMakeCache.txt")

        XCTAssertEqual(try runScript().status, 0)

        try FileManager.default.removeItem(at: project.appendingPathComponent("build/CMakeCache.txt"))
        let result = try runScript()
        XCTAssertEqual(result.status, 1)
        XCTAssertTrue(result.output.contains("Build the project first"))
    }

    func testInvalidInputsFailClearlyWithoutRotatingLogs() throws {
        try write("previous console", to: "mayaflux_run.txt")
        let cases: [([String], String)] = [
            (["--unsupported"], "Unknown option"),
            (["--executable"], "requires a value"),
            (["--configuration", "Fast"], "Choose Debug, Release"),
            (["--project-name", "../outside"], "project name is unsafe"),
            (["--project-name", "bad name"], "project name is unsafe"),
            (["--executable", "missing"], "missing or is not executable"),
            (["--executable", "build"], "missing or is not executable"),
            (["--mayaflux-root", root.appendingPathComponent("absent").path],
             "installation folder does not exist"),
            (["--mayaflux-root", root.path], "report configuration is missing"),
            (["--project-root", root.appendingPathComponent("absent").path],
             "project folder does not exist"),
        ]
        for (arguments, message) in cases {
            let result = try runScript(arguments: arguments)
            XCTAssertEqual(result.status, 1, "\(arguments): \(result.output)")
            XCTAssertTrue(result.output.contains(message), "\(arguments): \(result.output)")
        }
        XCTAssertEqual(try read("mayaflux_run.txt"), "previous console")
        XCTAssertFalse(FileManager.default.fileExists(
            atPath: project.appendingPathComponent("data/log/mayaflux_report.log").path))
    }

    func testMalformedReportConfigurationIsRejected() throws {
        try write("previous console", to: "mayaflux_run.txt")
        for content in ["{broken", "[1]", "", "\"text\""] {
            try content.write(to: config, atomically: true, encoding: .utf8)
            let result = try runScript()
            XCTAssertEqual(result.status, 1, content)
            XCTAssertTrue(result.output.contains("not a readable JSON object"), content)
        }
        XCTAssertEqual(try read("mayaflux_run.txt"), "previous console")
    }

    func testMissingMayaFluxRootIsRecoverable() throws {
        let result = try runScript(arguments: [], environment: ["MAYAFLUX_ROOT": ""], inheritRoot: false)

        XCTAssertEqual(result.status, 1, result.output)
        XCTAssertTrue(result.output.contains("MayaFlux was not found"))
        XCTAssertTrue(result.output.contains("MAYAFLUX_ROOT"))
    }

    func testMultipleConfigurationsRequireSelection() throws {
        try installApplication(at: "build/Release/ReportFixture")

        let ambiguous = try runScript()
        XCTAssertEqual(ambiguous.status, 1)
        XCTAssertTrue(ambiguous.output.contains("multiple project executables"))
        try FileManager.default.removeItem(at: project.appendingPathComponent("build/ReportFixture"))
        XCTAssertEqual(try runScript().status, 0)
        XCTAssertEqual(try runScript(arguments: ["--configuration", "Release"]).status, 0)
        XCTAssertEqual(try runScript(arguments: ["--configuration", "Debug"]).status, 1)
    }

    func testExplicitExecutableSupportsCustomLayoutWithSpaces() throws {
        try FileManager.default.removeItem(at: project.appendingPathComponent("build/ReportFixture"))
        try installApplication(at: "custom output/Renamed")

        let result = try runScript(arguments: ["--executable", "custom output/Renamed"])

        XCTAssertEqual(result.status, 0, result.output)
        XCTAssertTrue(try read("mayaflux_run.txt").contains("config=\(config.path)"))
    }

    func testUnstartableExecutableReportsErrorWithoutRotatingLogs() throws {
        try write("previous console", to: "mayaflux_run.txt")
        let broken = project.appendingPathComponent("build/Broken")
        try Data([0, 1, 2, 3]).write(to: broken)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: broken.path)

        let result = try runScript(arguments: ["--executable", "build/Broken"])

        XCTAssertEqual(result.status, 1, result.output)
        XCTAssertTrue(result.output.contains("[ERROR]"))
        XCTAssertTrue(result.output.contains("could not be started"))
        XCTAssertEqual(try read("mayaflux_run.txt"), "previous console")
    }

    private static var template: URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("templates/run-report-macos.sh")
    }

    private func writeExecutable(script: String, named name: String, in directory: URL) throws {
        let url = directory.appendingPathComponent(name)
        try script.write(to: url, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
    }

    private func installApplication(at relative: String) throws {
        let url = project.appendingPathComponent(relative)
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try writeExecutable(script: """
            #!/bin/bash
            echo "cwd=$(pwd -P)"
            echo "config=${2:-INVALID}"
            echo "environment=${WEAVE_REPORT_VALUE:-unset}"
            echo "stdout: diagnostic output"
            echo "stderr: crash details" >&2
            if [ "${WEAVE_REPORT_NO_LOG:-0}" != 1 ]; then
                mkdir -p data/log
                printf 'fresh application report' >data/log/mayaflux_report.log
            fi
            exit "${WEAVE_REPORT_EXIT:-0}"
            """, named: url.lastPathComponent, in: url.deletingLastPathComponent())
    }

    private func runScript(
        arguments: [String] = [], environment: [String: String] = [:], inheritRoot: Bool = true
    ) throws -> Result {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/bin/bash")
        process.arguments = [project.appendingPathComponent("run-report.sh").path] + arguments
        process.currentDirectoryURL = root
        var variables = ["PATH": "/usr/bin:/bin", "HOME": root.path]
        if inheritRoot { variables["MAYAFLUX_ROOT"] = install.path }
        process.environment = variables.merging(environment) { $1 }
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe
        try process.run()
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        return Result(status: process.terminationStatus, output: String(decoding: data, as: UTF8.self))
    }

    private func read(_ relative: String) throws -> String {
        try String(contentsOf: project.appendingPathComponent(relative), encoding: .utf8)
    }

    private func write(_ text: String, to relative: String) throws {
        let url = project.appendingPathComponent(relative)
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try text.write(to: url, atomically: true, encoding: .utf8)
    }

    private func previous(_ relative: String) throws -> [String] {
        let url = project.appendingPathComponent(relative)
        let name = url.lastPathComponent
        return try FileManager.default.contentsOfDirectory(
            at: url.deletingLastPathComponent(), includingPropertiesForKeys: nil
        ).filter { $0.lastPathComponent.hasPrefix(name + ".previous-") }
            .map { try String(contentsOf: $0, encoding: .utf8) }
    }
}
