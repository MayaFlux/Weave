import AppKit
import SwiftUI

// ============================================================================
// MARK: - App Entry
// ============================================================================

@main
struct WeaveApp: App {
    var body: some Scene {
        WindowGroup {
            LandingView()
        }
        .windowResizability(.contentSize)
    }
}

// ============================================================================
// MARK: - Mode
// ============================================================================

enum WeaveMode {
    case landing
    case install
    case projects
    case createProject
    case addCommunity
    case createCommunity
}

// ============================================================================
// MARK: - Landing View
// ============================================================================

struct LandingView: View {
    @State private var mode: WeaveMode = .landing

    var body: some View {
        switch mode {
        case .landing:
            VStack(spacing: 24) {
                HStack(spacing: 12) {
                    Image(systemName: "wand.and.stars")
                        .font(.system(size: 48))
                        .foregroundColor(.blue)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Weave")
                            .font(.largeTitle)
                            .fontWeight(.bold)
                        Text("MayaFlux Toolchain")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(.top, 10)

                Divider()

                VStack(spacing: 14) {
                    Button {
                        mode = .install
                    } label: {
                        Label("Install MayaFlux", systemImage: "arrow.down.circle")
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 6)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)

                    Button {
                        mode = .projects
                    } label: {
                        Label("Projects", systemImage: "folder")
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 6)
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.large)
                }

                Button("Quit") {
                    NSApplication.shared.terminate(nil)
                }
                .foregroundColor(.secondary)
                .padding(.bottom, 4)
            }
            .padding(32)
            .frame(width: 400)

        case .install:
            InstallView(onBack: { mode = .landing })

        case .projects:
            ProjectsView(
                onBack: { mode = .landing },
                onCreateProject: { mode = .createProject },
                onAddCommunity: { mode = .addCommunity },
                onCreateCommunity: { mode = .createCommunity })

        case .createProject:
            CreateProjectView(onBack: { mode = .landing })

        case .addCommunity:
            AddCommunityView(onBack: { mode = .projects })

        case .createCommunity:
            CreateCommunityView(onBack: { mode = .projects })
        }
    }
}

// ============================================================================
// MARK: - Install Step
// ============================================================================

// ============================================================================
// MARK: - Installer State
// ============================================================================

@MainActor
class InstallerState: ObservableObject {
    @Published var step: InstallStep = .channelSelect
    @Published var formula: String = "mayaflux"
    @Published var password: String = ""
    @Published var logLines: [String] = []
    @Published var createdZshenvPath: String? = nil

    var brewCmd: String? {
        let optDirectories = (try? FileManager.default.contentsOfDirectory(atPath: "/opt"))
            ?? []
        return HomebrewLocator.discover(
            path: ProcessInfo.processInfo.environment["PATH"] ?? "",
            optDirectories: optDirectories,
            isExecutable: FileManager.default.isExecutableFile(atPath:))
    }

    var brewMissing: Bool { brewCmd == nil }

    func log(_ line: String) {
        logLines.append(line)
    }

    func shellEscape(_ s: String) -> String {
        "'" + s.replacingOccurrences(of: "'", with: "'\\''") + "'"
    }

    func run(_ command: String) async -> Int32 {
        return await withCheckedContinuation { continuation in
            let task = Process()
            task.executableURL = URL(fileURLWithPath: "/bin/zsh")
            task.arguments = ["-c", command]

            var env = ProcessInfo.processInfo.environment
            if !password.isEmpty {
                env["SUDO_PASSWORD"] = password
            }
            task.environment = env

            let pipe = Pipe()
            task.standardOutput = pipe
            task.standardError = pipe

            pipe.fileHandleForReading.readabilityHandler = { handle in
                let data = handle.availableData
                guard !data.isEmpty, let text = String(data: data, encoding: .utf8) else { return }
                let lines = text.components(separatedBy: .newlines).filter { !$0.isEmpty }
                DispatchQueue.main.async { self.logLines.append(contentsOf: lines) }
            }

            task.terminationHandler = { t in
                pipe.fileHandleForReading.readabilityHandler = nil
                continuation.resume(returning: t.terminationStatus)
            }

            do {
                try task.run()
            } catch {
                DispatchQueue.main.async {
                    self.logLines.append("ERROR: \(error.localizedDescription)")
                }
                continuation.resume(returning: 1)
            }
        }
    }

    func validatePassword() async -> Bool {
        let code = await run("echo \(shellEscape(password)) | sudo -S -v 2>/dev/null")
        return code == 0
    }

    func conflictingFormula() -> String {
        formula == "mayaflux" ? "mayaflux-dev" : "mayaflux"
    }

    func checkConflict(brew: String) async -> Bool {
        let code = await run(
            "\(shellEscape(brew)) list --formula | grep -E '^" + conflictingFormula() + "$'")
        return code == 0
    }

    // ============================================================
    // MARK: - Install pipeline
    // ============================================================

    func startInstall() async {
        step = .running
        logLines = []
        createdZshenvPath = nil
        step = await InstallerWorkflow.run(
            formula: formula,
            findBrew: { self.brewCmd },
            runCommand: { command in await self.run(command) },
            configureEnvironment: { brew, _ in
                await self.configureEnvironment(brew: brew)
                if case .failed(let message) = self.step { return message }
                return nil
            },
            log: { self.log($0) })
    }

    func configureEnvironment(brew: String) async {
        log("➤ Configuring environment...")

        // Get prefix
        let prefixTask = Process()
        prefixTask.executableURL = URL(fileURLWithPath: brew)
        prefixTask.arguments = ["--prefix", formula]
        let prefixPipe = Pipe()
        prefixTask.standardOutput = prefixPipe
        do {
            try prefixTask.run()
        } catch {
            step = .failed("Could not determine Homebrew prefix for \(formula).")
            return
        }
        prefixTask.waitUntilExit()
        let prefixData = prefixPipe.fileHandleForReading.readDataToEndOfFile()
        let prefixOutput = (String(data: prefixData, encoding: .utf8) ?? "").trimmingCharacters(
            in: .whitespacesAndNewlines)

        guard prefixTask.terminationStatus == 0, !prefixOutput.isEmpty else {
            step = .failed("Could not determine Homebrew prefix for \(formula).")
            return
        }
        log("  Prefix: \(prefixOutput)")

        let zdotdirTask = Process()
        zdotdirTask.executableURL = URL(fileURLWithPath: "/bin/zsh")
        zdotdirTask.arguments = ["-lc", "printf %s \"${ZDOTDIR:-}\""]
        let zdotdirPipe = Pipe()
        zdotdirTask.standardOutput = zdotdirPipe
        let zdotdirRaw: String
        do {
            try zdotdirTask.run()
            zdotdirTask.waitUntilExit()
            zdotdirRaw = String(
                data: zdotdirPipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8
            )?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        } catch {
            zdotdirRaw = ""
        }

        let home = URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
        let zshenvURL = ShellEnvironmentFile.resolvePath(home: home, zdotdir: zdotdirRaw)
        if zdotdirRaw.isEmpty {
            log("  ZDOTDIR not set, falling back to HOME")
        } else if zshenvURL.deletingLastPathComponent().path == home.path {
            log("  ZDOTDIR='\(zdotdirRaw)' does not exist, falling back to HOME")
        }

        log("  zshenv: \(zshenvURL.path)")
        let existed = FileManager.default.fileExists(atPath: zshenvURL.path)
        let backupURL = zshenvURL.appendingPathExtension("weave-backup")
        if existed,
            let contents = try? String(contentsOf: zshenvURL, encoding: .utf8),
            !contents.contains("# MayaFlux")
        {
            log("  Backup created: \(backupURL.path)")
        }
        do {
            let created = try ShellEnvironmentFile.update(at: zshenvURL, brewPrefix: prefixOutput)
            if created {
                createdZshenvPath = zshenvURL.path
                log("  Created \(zshenvURL.path)")
            }
        } catch {
            step = .failed("Failed to update \(zshenvURL.path): \(error.localizedDescription)")
            return
        }

        log("✅ Environment configured")

        // --- Weave CLI ---
        log("➤ Installing Weave CLI...")
        let localBin = NSHomeDirectory() + "/.local/bin"
        let weaveDest = localBin + "/weave"

        guard let resourcePath = Bundle.main.resourcePath else {
            step = .failed("Could not locate app bundle resources.")
            return
        }
        let weaveSource = resourcePath + "/weave"

        let cliCode = await run(
            "mkdir -p \(shellEscape(localBin)) && "
                + "cp \(shellEscape(weaveSource)) \(shellEscape(weaveDest)) && "
                + "chmod +x \(shellEscape(weaveDest))"
        )
        guard cliCode == 0 else {
            step = .failed("Failed to install Weave CLI to \(weaveDest).")
            return
        }

        log("Installing templates...")
        let templatesSource = resourcePath + "/templates"
        let templatesDest = NSHomeDirectory() + "/.local/share/weave/templates"
        let templatesCode = await run(
            "test -f \(shellEscape(templatesSource + "/validate-macos.sh")) && "
                + "mkdir -p \(shellEscape(templatesDest)) && "
                + "cp -R \(shellEscape(templatesSource))/ \(shellEscape(templatesDest))/ && "
                + "test -f \(shellEscape(templatesDest + "/validate-macos.sh"))"
        )
        guard templatesCode == 0 else {
            step = .failed("Failed to install templates to \(templatesDest).")
            return
        }
        log("Templates installed")

        log("✅ Weave CLI installed to \(weaveDest)")

        step = .done
    }
}

// ============================================================================
// MARK: - Projects View
// ============================================================================

struct ProjectsView: View {
    let onBack: () -> Void
    let onCreateProject: () -> Void
    let onAddCommunity: () -> Void
    let onCreateCommunity: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button(action: onBack) {
                    Label("Back", systemImage: "chevron.left")
                }
                .buttonStyle(.borderless)
                Spacer()
                Text("Projects").font(.headline)
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)

            Divider()

            VStack(spacing: 14) {
                Button {
                    onCreateProject()
                } label: {
                    Label("Create Project", systemImage: "folder.badge.plus")
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 6)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)

                Button {
                    onAddCommunity()
                } label: {
                    Label("Update Project (Community Modules)", systemImage: "arrow.down.circle")
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 6)
                }
                .buttonStyle(.bordered)
                .controlSize(.large)

                Button {
                    onCreateCommunity()
                } label: {
                    Label("Create Community Module", systemImage: "puzzlepiece")
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 6)
                }
                .buttonStyle(.bordered)
                .controlSize(.large)
            }
            .padding(24)

            Spacer()
        }
        .frame(width: 400)
    }
}

// ============================================================================
// MARK: - Install View
// ============================================================================

struct InstallView: View {
    let onBack: () -> Void

    @StateObject private var state = InstallerState()
    @State private var showConflictAlert = false
    @State private var conflictingFormula = ""

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button(action: onBack) {
                    Label("Back", systemImage: "chevron.left")
                }
                .buttonStyle(.borderless)
                .disabled(
                    {
                        if case .running = state.step { return true }
                        return false
                    }())
                Spacer()
                Text("Install MayaFlux").font(.headline)
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)

            Divider()

            switch state.step {
            case .channelSelect: channelSelectBody
            case .passwordPrompt: passwordPromptBody
            case .running: runningBody
            case .done: doneBody
            case .failed(let m): failedBody(m)
            }
        }
        .frame(width: 580, height: 480)
        .alert("Conflicting Installation", isPresented: $showConflictAlert) {
            Button("Cancel", role: .cancel) {}
            Button("Remove \(conflictingFormula) and continue", role: .destructive) {
                Task { await state.startInstall() }
            }
        } message: {
            Text(
                "You have \(conflictingFormula) installed. Both versions cannot coexist.\n\nRemove it and install \(state.formula)?"
            )
        }
    }

    // ---- Channel Select ----

    var channelSelectBody: some View {
        VStack(spacing: 24) {
            Spacer()
            Text("Select Release Channel")
                .font(.title2).fontWeight(.semibold)

            VStack(spacing: 12) {
                channelButton(
                    label: "Stable", description: "Recommended for most users",
                    icon: "checkmark.seal", selected: state.formula == "mayaflux"
                ) {
                    state.formula = "mayaflux"
                }
                channelButton(
                    label: "Development", description: "Latest features, may be unstable",
                    icon: "bolt", selected: state.formula == "mayaflux-dev"
                ) {
                    state.formula = "mayaflux-dev"
                }
            }
            .padding(.horizontal, 40)

            Text(
                "Weave will add the mayaflux/mayaflux Homebrew tap and trust it when supported. Trust allows Homebrew to load its current and future formulae, casks, and commands."
            )
            .font(.caption)
            .foregroundColor(.secondary)
            .multilineTextAlignment(.center)
            .padding(.horizontal, 40)

            Spacer()

            Button("Continue") { advanceFromChannelSelect() }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .padding(.bottom, 24)
        }
    }

    func channelButton(
        label: String, description: String, icon: String, selected: Bool,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 16) {
                Image(systemName: icon)
                    .font(.system(size: 22))
                    .foregroundColor(selected ? .white : .blue)
                    .frame(width: 32)
                VStack(alignment: .leading, spacing: 2) {
                    Text(label).fontWeight(.semibold)
                        .foregroundColor(selected ? .white : .primary)
                    Text(description).font(.caption)
                        .foregroundColor(selected ? .white.opacity(0.85) : .secondary)
                }
                Spacer()
                if selected {
                    Image(systemName: "checkmark.circle.fill").foregroundColor(.white)
                }
            }
            .padding(14)
            .background(selected ? Color.blue : Color(NSColor.controlBackgroundColor))
            .cornerRadius(10)
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(selected ? Color.clear : Color.gray.opacity(0.3), lineWidth: 1))
        }
        .buttonStyle(.plain)
    }

    // ---- Password Prompt ----

    var passwordPromptBody: some View {
        VStack(spacing: 20) {
            Spacer()
            Image(systemName: "lock.circle").font(.system(size: 52)).foregroundColor(.blue)
            Text("Administrator Password Required").font(.title2).fontWeight(.semibold)
            Text(
                "Homebrew is not installed. Installing it requires your password once to set up its directory.\n\nYour password is not stored."
            )
            .multilineTextAlignment(.center)
            .foregroundColor(.secondary)
            .padding(.horizontal, 40)
            SecureField("Password", text: $state.password)
                .textFieldStyle(.roundedBorder)
                .padding(.horizontal, 60)
                .onSubmit { confirmPasswordAndContinue() }
            Spacer()
            HStack(spacing: 16) {
                Button("Back") { state.step = .channelSelect }.buttonStyle(.bordered)
                Button("Continue") { confirmPasswordAndContinue() }
                    .buttonStyle(.borderedProminent)
                    .disabled(state.password.isEmpty)
            }
            .padding(.bottom, 24)
        }
        .frame(maxWidth: .infinity)
    }

    // ---- Running ----

    var runningBody: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(Array(state.logLines.enumerated()), id: \.offset) { idx, line in
                        Text(line)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(logColor(line))
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .id(idx)
                    }
                }
                .padding(12)
            }
            .background(Color(NSColor.textBackgroundColor))
            .padding(.horizontal, 20)
            .padding(.vertical, 16)
            .onChange(of: state.logLines.count) {
                if let last = state.logLines.indices.last {
                    proxy.scrollTo(last, anchor: .bottom)
                }
            }
        }
    }

    func logColor(_ line: String) -> Color {
        if line.hasPrefix("ERROR") { return .red }
        if line.hasPrefix("✅") || line.hasPrefix("✓") { return .green }
        if line.hasPrefix("⚠️") { return .orange }
        if line.hasPrefix("➤") { return .blue }
        return .primary
    }

    // ---- Done ----

    var doneBody: some View {
        VStack(spacing: 20) {
            Spacer()
            Image(systemName: "checkmark.circle.fill").font(.system(size: 64)).foregroundColor(
                .green)
            Text("Installation Complete!").font(.title).fontWeight(.bold)
            Text("Restart your terminal for environment changes to take effect.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
                .padding(.horizontal, 40)
            if let createdPath = state.createdZshenvPath {
                Text(
                    "Weave created \(createdPath) and added the MayaFlux setup. If your .zshenv is elsewhere, copy the MayaFlux block from this file into yours, or set ZDOTDIR to your config directory and rerun Weave."
                )
                .font(.caption)
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
                .padding(.horizontal, 30)
            }
            VStack(alignment: .leading, spacing: 6) {
                Text("source ${ZDOTDIR:-$HOME}/.zshenv")
                    .font(.system(.body, design: .monospaced))
                Text("weave new MyProject ~/Projects/")
                    .font(.system(.body, design: .monospaced))
            }
            .padding(14)
            .background(Color(NSColor.textBackgroundColor))
            .cornerRadius(8)
            Spacer()
            Button("Done") { NSApplication.shared.terminate(nil) }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .padding(.bottom, 24)
        }
        .frame(maxWidth: .infinity)
    }

    // ---- Failed ----

    func failedBody(_ message: String) -> some View {
        VStack(spacing: 20) {
            Spacer()
            Image(systemName: "xmark.circle.fill").font(.system(size: 64)).foregroundColor(.red)
            Text("Installation Failed").font(.title).fontWeight(.bold)
            ScrollView {
                Text(message)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.red)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(12)
            }
            .background(Color(NSColor.textBackgroundColor))
            .cornerRadius(8)
            .frame(maxHeight: 160)
            .padding(.horizontal, 24)
            Text("Full log: ~/.cache/weave_install.log")
                .font(.caption).foregroundColor(.secondary)
            Spacer()
            HStack(spacing: 16) {
                Button("Back") { onBack() }.buttonStyle(.bordered)
                Button("Retry") {
                    state.logLines = []
                    state.step = .channelSelect
                }.buttonStyle(.borderedProminent)
            }
            .padding(.bottom, 24)
        }
        .frame(maxWidth: .infinity)
    }

    // ============================================================
    // MARK: - Flow control
    // ============================================================

    func advanceFromChannelSelect() {
        if state.brewMissing {
            state.step = .passwordPrompt
        } else {
            checkConflictThenInstall()
        }
    }

    func confirmPasswordAndContinue() {
        guard !state.password.isEmpty else { return }
        Task {
            let valid = await state.validatePassword()
            if valid {
                checkConflictThenInstall()
            } else {
                state.password = ""
            }
        }
    }

    func checkConflictThenInstall() {
        guard let brew = state.brewCmd else {
            Task { await state.startInstall() }
            return
        }
        let conflicting = state.conflictingFormula()
        Task {
            let code = await state.run("\(brew) list --formula | grep -E '^" + conflicting + "$'")
            if code == 0 {
                conflictingFormula = conflicting
                showConflictAlert = true
            } else {
                await state.startInstall()
            }
        }
    }
}

// ============================================================================
// MARK: - Create Project View
// ============================================================================

struct CreateProjectView: View {
    let onBack: () -> Void

    @State private var projectName: String = "MyFirstProject"
    @State private var projectLocation: String = NSHomeDirectory() + "/Projects"
    @State private var withLila: Bool = false
    @State private var isCreating: Bool = false
    @State private var showAlert: Bool = false
    @State private var alertMessage: String = ""

    private var weaveURL: URL? {
        ProjectCreation.resolveExecutable(
            installed: URL(fileURLWithPath: "\(NSHomeDirectory())/.local/bin/weave"),
            bundled: URL(fileURLWithPath: "\(Bundle.main.resourcePath ?? "")/weave"),
            fileExists: FileManager.default.fileExists(atPath:))
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button(action: onBack) {
                    Label("Back", systemImage: "chevron.left")
                }
                .buttonStyle(.borderless)
                .disabled(isCreating)
                Spacer()
                Text("Create New Project").font(.headline)
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)

            Divider()

            VStack(spacing: 20) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Project Name").font(.headline)
                    TextField("MyFirstProject", text: $projectName)
                        .textFieldStyle(.roundedBorder)
                }

                VStack(alignment: .leading, spacing: 8) {
                    Text("Project Location").font(.headline)
                    HStack {
                        TextField("/path/to/location", text: $projectLocation)
                            .textFieldStyle(.roundedBorder)
                        Button("Browse…") { selectFolder() }
                    }
                }

                Toggle("Enable Lila (Live Coding)", isOn: $withLila)
                    .toggleStyle(CheckboxToggleStyle())

                Divider()

                VStack(alignment: .leading, spacing: 4) {
                    Text("Project will be created at:")
                        .font(.caption).foregroundColor(.secondary)
                    Text("\(projectLocation)/\(projectName)")
                        .font(.system(.body, design: .monospaced))
                        .foregroundColor(.blue)
                }
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color(NSColor.textBackgroundColor))
                .cornerRadius(8)

                Spacer()

                HStack(spacing: 16) {
                    Button("Cancel") { NSApplication.shared.terminate(nil) }
                        .keyboardShortcut(.cancelAction)
                    Button(isCreating ? "Creating…" : "Create Project") { createProject() }
                        .disabled(isCreating || projectName.isEmpty)
                        .buttonStyle(.borderedProminent)
                        .keyboardShortcut(.defaultAction)
                }
            }
            .padding(24)
        }
        .frame(width: 550, height: 450)
        .alert("Weave", isPresented: $showAlert) {
            Button("OK") {
                if alertMessage.contains("successfully") {
                    NSApplication.shared.terminate(nil)
                }
            }
        } message: {
            Text(alertMessage)
        }
    }

    func selectFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = true
        panel.message = "Select project location"
        if panel.runModal() == .OK, let url = panel.url {
            projectLocation = url.path
        }
    }

    func createProject() {
        guard !projectName.isEmpty else { return }
        isCreating = true

        guard let weaveURL else {
            alertMessage =
                "Weave CLI was not found in the app bundle or at ~/.local/bin/weave.\n\nPlease run Install MayaFlux first."
            showAlert = true
            isCreating = false
            return
        }

        let task = Process()
        task.executableURL = weaveURL
        task.arguments = ProjectCreation.arguments(
            name: projectName, location: projectLocation, withLila: withLila)

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = pipe

        do {
            try task.run()
            task.waitUntilExit()

            if task.terminationStatus == 0 {
                alertMessage = """
                    Project '\(projectName)' created successfully!

                    Location: \(projectLocation)/\(projectName)

                    Next steps:
                    1. cd \(projectLocation)/\(projectName)
                    2. mkdir build && cd build
                    3. cmake .. && make
                    4. ./\(projectName)
                    """
            } else {
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                let output = String(data: data, encoding: .utf8) ?? "Unknown error"
                alertMessage = "Failed to create project:\n\n\(output)"
            }
        } catch {
            alertMessage = "Error: \(error.localizedDescription)"
        }

        showAlert = true
        isCreating = false
    }
}

// ============================================================================
// MARK: - Create Community View
// ============================================================================

struct CreateCommunityView: View {
    let onBack: () -> Void

    @State private var moduleName: String = ""
    @State private var moduleNameError: String = ""
    @State private var description: String = ""
    @State private var minVersion: String = "0.5.0"
    @State private var needsLila: Bool = false
    @State private var destination: String = NSHomeDirectory() + "/Projects"
    @State private var isCreating: Bool = false
    @State private var showAlert: Bool = false
    @State private var alertMessage: String = ""

    private var weaveURL: URL? {
        ProjectCreation.resolveExecutable(
            installed: URL(fileURLWithPath: "\(NSHomeDirectory())/.local/bin/weave"),
            bundled: URL(fileURLWithPath: "\(Bundle.main.resourcePath ?? "")/weave"),
            fileExists: FileManager.default.fileExists(atPath:))
    }

    private var preview: String {
        "\(destination)/\(moduleName.isEmpty ? "my_module" : moduleName)"
    }

    private var nameIsValid: Bool {
        let r = try! NSRegularExpression(pattern: "^[a-z][a-z0-9_]*$")
        return r.firstMatch(
            in: moduleName, range: NSRange(moduleName.startIndex..., in: moduleName)) != nil
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button(action: onBack) {
                    Label("Back", systemImage: "chevron.left")
                }
                .buttonStyle(.borderless)
                .disabled(isCreating)
                Spacer()
                Text("Create Community Module").font(.headline)
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Module Name (snake_case)").font(.headline)
                        TextField("my_module", text: $moduleName)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: moduleName) { _ in validateName() }
                        if !moduleNameError.isEmpty {
                            Text(moduleNameError)
                                .font(.caption)
                                .foregroundColor(.red)
                        }
                    }

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Description").font(.headline)
                        TextField(
                            "A short description of what this module does", text: $description
                        )
                        .textFieldStyle(.roundedBorder)
                    }

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Minimum MayaFlux Version").font(.headline)
                        TextField("0.5.0", text: $minVersion)
                            .textFieldStyle(.roundedBorder)
                    }

                    Toggle("Requires Lila (live coding)", isOn: $needsLila)
                        .toggleStyle(CheckboxToggleStyle())

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Destination").font(.headline)
                        HStack {
                            TextField("/path/to/location", text: $destination)
                                .textFieldStyle(.roundedBorder)
                            Button("Browse…") { selectFolder() }
                        }
                    }

                    Divider()

                    VStack(alignment: .leading, spacing: 4) {
                        Text("Module will be created at:")
                            .font(.caption).foregroundColor(.secondary)
                        Text(preview)
                            .font(.system(.body, design: .monospaced))
                            .foregroundColor(.blue)
                    }
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color(NSColor.textBackgroundColor))
                    .cornerRadius(8)
                }
                .padding(24)
            }

            Divider()

            HStack(spacing: 16) {
                Button("Cancel") { NSApplication.shared.terminate(nil) }
                    .keyboardShortcut(.cancelAction)
                Button(isCreating ? "Creating…" : "Create Module") { createModule() }
                    .disabled(isCreating || moduleName.isEmpty || !nameIsValid)
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
            }
            .padding(16)
        }
        .frame(width: 550, height: 480)
        .alert("Weave", isPresented: $showAlert) {
            Button("OK") {
                if alertMessage.contains("created") {
                    NSApplication.shared.terminate(nil)
                }
            }
        } message: {
            Text(alertMessage)
        }
    }

    func validateName() {
        guard !moduleName.isEmpty else {
            moduleNameError = ""
            return
        }
        moduleNameError =
            nameIsValid
            ? ""
            : "Must be snake_case: lowercase letters, digits, underscores, no leading digit"
    }

    func selectFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = true
        panel.message = "Select destination"
        if panel.runModal() == .OK, let url = panel.url {
            destination = url.path
        }
    }

    func createModule() {
        guard !moduleName.isEmpty, nameIsValid else { return }
        isCreating = true

        guard let weaveURL else {
            alertMessage =
                "Weave CLI was not found in the app bundle or at ~/.local/bin/weave.\n\nPlease run Install MayaFlux first."
            showAlert = true
            isCreating = false
            return
        }

        let task = Process()
        task.executableURL = weaveURL
        task.arguments = ["community", moduleName, destination]

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = pipe

        do {
            let adjacentTemplates = weaveURL
                .deletingLastPathComponent().appendingPathComponent("templates")
            let sharedTemplates = URL(fileURLWithPath: NSHomeDirectory())
                .appendingPathComponent(".local/share/weave/templates")
            var isDirectory: ObjCBool = false
            let templateDirectory: URL

            if FileManager.default.fileExists(
                atPath: adjacentTemplates.path, isDirectory: &isDirectory),
                isDirectory.boolValue
            {
                templateDirectory = adjacentTemplates
            } else {
                templateDirectory = sharedTemplates
            }

            let templateURL = templateDirectory.appendingPathComponent("community/community.json")
            let templateSource = try String(contentsOf: templateURL, encoding: .utf8)
                .replacingOccurrences(of: "@MODULE_NAME@", with: moduleName)
            _ = try CommunityManifest.parse(templateSource, expectedName: moduleName)

            try task.run()
            task.waitUntilExit()

            if task.terminationStatus == 0 {
                let moduleDir = URL(fileURLWithPath: destination).appendingPathComponent(moduleName)
                let generatedManifest = try String(
                    contentsOf: moduleDir.appendingPathComponent("community.json"), encoding: .utf8)
                try CommunityModuleCreation.apply(
                    source: generatedManifest,
                    expectedName: moduleName,
                    description: description,
                    minVersion: minVersion,
                    needsLila: needsLila,
                    moduleDirectory: moduleDir)
                alertMessage =
                    "Module '\(moduleName)' created successfully!\n\nLocation: \(moduleDir.path)"
            } else {
                let output =
                    String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)
                    ?? "Unknown error"
                alertMessage = "Failed to create module:\n\n\(output)"
            }
        } catch {
            alertMessage = "Error: \(error.localizedDescription)"
        }

        showAlert = true
        isCreating = false
    }
}

// ============================================================================
// MARK: - Add Community View
// ============================================================================

struct AddCommunityView: View {
    let onBack: () -> Void

    @State private var projectPath: String = NSHomeDirectory() + "/Projects"
    @State private var moduleName: String = ""
    @State private var moduleNameError: String = ""
    @State private var isAdding: Bool = false
    @State private var output: String = ""
    @State private var showAlert: Bool = false
    @State private var alertMessage: String = ""

    private var nameIsValid: Bool {
        (try? CommunityUpdate.names([moduleName])) != nil
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button(action: onBack) {
                    Label("Back", systemImage: "chevron.left")
                }
                .buttonStyle(.borderless)
                .disabled(isAdding)
                Spacer()
                Text("Add Community Module").font(.headline)
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Project Directory").font(.headline)
                        HStack {
                            TextField("/path/to/project", text: $projectPath)
                                .textFieldStyle(.roundedBorder)
                            Button("Browse…") {
                                let panel = NSOpenPanel()
                                panel.canChooseFiles = false
                                panel.canChooseDirectories = true
                                panel.allowsMultipleSelection = false
                                if let path = projectPath.isEmpty
                                    ? nil : URL(fileURLWithPath: projectPath)
                                {
                                    panel.directoryURL = path
                                }
                                if panel.runModal() == .OK, let url = panel.url {
                                    projectPath = url.path
                                }
                            }
                        }
                    }

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Module Names").font(.headline)
                        TextField("module_one, module_two", text: $moduleName)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: moduleName) { _ in
                                moduleNameError =
                                    (!moduleName.isEmpty && !nameIsValid)
                                    ? "Use comma-separated snake_case module names"
                                    : ""
                            }
                        if !moduleNameError.isEmpty {
                            Text(moduleNameError).font(.caption).foregroundColor(.red)
                        }
                    }

                    VStack(alignment: .leading, spacing: 6) {
                        Text("Output").font(.headline)
                        ScrollView {
                            Text(output.isEmpty ? " " : output)
                                .font(.system(.caption, design: .monospaced))
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(8)
                        }
                        .frame(height: 180)
                        .background(Color(NSColor.textBackgroundColor))
                        .cornerRadius(6)
                        .overlay(RoundedRectangle(cornerRadius: 6).stroke(Color.gray.opacity(0.3)))
                    }
                }
                .padding(20)
            }

            Divider()

            HStack {
                Button("Cancel", action: onBack).disabled(isAdding)
                Spacer()
                Button(isAdding ? "Adding…" : "Add Module") {
                    addModule()
                }
                .buttonStyle(.borderedProminent)
                .disabled(isAdding || moduleName.isEmpty || !nameIsValid)
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
        }
        .frame(width: 520, height: 500)
        .alert("Done", isPresented: $showAlert) {
            Button("OK") { onBack() }
        } message: {
            Text(alertMessage)
        }
    }

    private func addModule() {
        guard !projectPath.isEmpty, !moduleName.isEmpty, nameIsValid else { return }

        isAdding = true
        output = ""

        Task.detached(priority: .userInitiated) {
            do {
                try await CommunityUpdate.run(
                    projectPath: projectPath,
                    values: [moduleName],
                    report: { line in
                        DispatchQueue.main.async {
                            output += line + "\n"
                        }
                    }
                )

                await MainActor.run {
                    isAdding = false
                    alertMessage = "Modules added. Rebuild your project to include them."
                    showAlert = true
                }
            } catch {
                await MainActor.run {
                    output += "Error: \(error.localizedDescription)\n"
                    isAdding = false
                }
            }
        }
    }
}

// ============================================================================
// MARK: - Shared
// ============================================================================

struct CheckboxToggleStyle: ToggleStyle {
    func makeBody(configuration: Configuration) -> some View {
        HStack {
            Image(systemName: configuration.isOn ? "checkmark.square.fill" : "square")
                .foregroundColor(configuration.isOn ? .accentColor : .gray)
                .onTapGesture { configuration.isOn.toggle() }
            configuration.label
        }
    }
}
