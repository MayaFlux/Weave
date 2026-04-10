import SwiftUI
import AppKit

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
    case createProject
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
                        mode = .createProject
                    } label: {
                        Label("Create New Project", systemImage: "folder.badge.plus")
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

        case .createProject:
            CreateProjectView(onBack: { mode = .landing })
        }
    }
}

// ============================================================================
// MARK: - Install Step
// ============================================================================

enum InstallStep {
    case channelSelect
    case passwordPrompt
    case running
    case done
    case failed(String)
}

// ============================================================================
// MARK: - Installer State
// ============================================================================

@MainActor
class InstallerState: ObservableObject {
    @Published var step: InstallStep = .channelSelect
    @Published var formula: String = "mayaflux"
    @Published var password: String = ""
    @Published var logLines: [String] = []

    var brewCmd: String? {
        for p in ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
            if FileManager.default.isExecutableFile(atPath: p) { return p }
        }
        return nil
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
                DispatchQueue.main.async { self.logLines.append("ERROR: \(error.localizedDescription)") }
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
        let code = await run("\(brew) list --formula | grep -E '^" + conflictingFormula() + "$'")
        return code == 0
    }

    // ============================================================
    // MARK: - Install pipeline
    // ============================================================

    func startInstall() async {
        step = .running
        logLines = []

        // --- Homebrew ---
        let brew: String
        if let found = brewCmd {
            brew = found
            log("✅ Homebrew found at \(brew)")
        } else {
            log("➤ Installing Homebrew...")
            log("  This may take a few minutes...")
            let code = await run(
                "NONINTERACTIVE=1 /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            )
            guard code == 0 else {
                step = .failed("Homebrew installation failed.\n\nCheck your internet connection and try again.")
                return
            }
            guard let found = brewCmd else {
                step = .failed("Homebrew installed but executable not found.\nExpected: /opt/homebrew/bin/brew or /usr/local/bin/brew")
                return
            }
            brew = found
            log("✅ Homebrew installed at \(brew)")
        }

        // --- Conflict removal (user already confirmed via alert before we got here) ---
        let conflicting = conflictingFormula()
        let conflictCode = await run("\(brew) list --formula | grep -E '^" + conflicting + "$'")
        if conflictCode == 0 {
            log("➤ Removing \(conflicting)...")
            let removeCode = await run("\(brew) uninstall \(conflicting)")
            guard removeCode == 0 else {
                step = .failed("Failed to remove \(conflicting).")
                return
            }
            log("✅ \(conflicting) removed")
        }

        // --- Tap ---
        log("➤ Adding tap mayaflux/mayaflux...")
        let tapCode = await run("\(brew) tap mayaflux/mayaflux")
        guard tapCode == 0 else {
            step = .failed("Failed to add mayaflux tap.\n\nCheck your internet connection.")
            return
        }
        log("✅ Tap added")

        // --- Install ---
        log("➤ Installing \(formula)...")
        log("  ⏳ This may take several minutes...")
        let installCode = await run("\(brew) install \(formula)")
        guard installCode == 0 else {
            step = .failed("brew install \(formula) failed.\n\nSee log above for details.")
            return
        }
        log("✅ \(formula) installed")

        // --- Verify ---
        log("➤ Verifying installation...")
        let verifyCode = await run("\(brew) list --formula | grep -E '^" + formula + "$'")
        guard verifyCode == 0 else {
            step = .failed("Installation verification failed — \(formula) not found in brew list.")
            return
        }
        log("✅ Verified")

        // --- Environment (always runs unless install itself failed) ---
        await configureEnvironment(brew: brew)
    }

    func configureEnvironment(brew: String) async {
        log("➤ Configuring environment...")

        // Get prefix
        let prefixTask = Process()
        prefixTask.executableURL = URL(fileURLWithPath: brew)
        prefixTask.arguments = ["--prefix", formula]
        let prefixPipe = Pipe()
        prefixTask.standardOutput = prefixPipe
        try? prefixTask.run()
        prefixTask.waitUntilExit()
        let prefixData = prefixPipe.fileHandleForReading.readDataToEndOfFile()
        let prefixOutput = (String(data: prefixData, encoding: .utf8) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)

        guard !prefixOutput.isEmpty else {
            step = .failed("Could not determine Homebrew prefix for \(formula).")
            return
        }
        log("  Prefix: \(prefixOutput)")

        // Ask zsh (login shell) for ZDOTDIR — Swift's environment is bare when launched from Finder
        let zdotdirTask = Process()
        zdotdirTask.executableURL = URL(fileURLWithPath: "/bin/zsh")
        zdotdirTask.arguments = ["-lc", "echo $ZDOTDIR"]
        let zdotdirPipe = Pipe()
        zdotdirTask.standardOutput = zdotdirPipe
        try? zdotdirTask.run()
        zdotdirTask.waitUntilExit()
        let zdotdirRaw = (String(data: zdotdirPipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)

        let zshenvDir: String
        if zdotdirRaw.isEmpty {
            log("  ZDOTDIR not set, falling back to HOME")
            zshenvDir = NSHomeDirectory()
        } else if !FileManager.default.fileExists(atPath: zdotdirRaw) {
            log("  ZDOTDIR='\(zdotdirRaw)' does not exist, falling back to HOME")
            zshenvDir = NSHomeDirectory()
        } else {
            zshenvDir = zdotdirRaw
        }

        let zshenvPath = zshenvDir + "/.zshenv"
        log("  zshenv: \(zshenvPath)")

        let fileExists = FileManager.default.fileExists(atPath: zshenvPath)

        // Backup only if file exists and has never been touched by us
        if fileExists {
            let contents = (try? String(contentsOfFile: zshenvPath, encoding: .utf8)) ?? ""
            if !contents.contains("# MayaFlux") {
                let backupPath = zshenvPath + ".weave-backup"
                _ = await run("cp \(shellEscape(zshenvPath)) \(shellEscape(backupPath))")
                log("  Backup created: \(backupPath)")
            }
        }

        // Strip old entries — BSD sed -i '' verbatim
        let escaped = shellEscape(zshenvPath)
        let cleanScript =
            "[ -f \(escaped) ] && sed -i '' '/# MayaFlux/d' \(escaped);" +
            "[ -f \(escaped) ] && sed -i '' '/MAYAFLUX_ROOT/d' \(escaped);" +
            "[ -f \(escaped) ] && sed -i '' '/mayaflux_env\\.sh/d' \(escaped);" +
            "[ -f \(escaped) ] && sed -i '' '/source.*mayaflux.*\\/env\\.sh/d' \(escaped);" +
            "[ -f \(escaped) ] && sed -i '' '/\\.local\\/bin.*\\$PATH/d' \(escaped);" +
            "[ -f \(escaped) ] && sed -i '' '/./,$!d' \(escaped);" +
            "true"

        let cleanCode = await run(cleanScript)
        guard cleanCode == 0 else {
            step = .failed("Failed to clean old MayaFlux config from \(zshenvPath)")
            return
        }

        // If file does not exist, refuse to create it
        guard fileExists else {
            step = .failed(
                "No .zshenv found at \(zshenvPath)\n\n" +
                "MayaFlux cannot configure your shell environment without an existing .zshenv.\n\n" +
                "Create one first:\n  touch \(zshenvPath)"
            )
            return
        }

        guard let handle = FileHandle(forWritingAtPath: zshenvPath) else {
            step = .failed("Failed to open \(zshenvPath) for writing.")
            return
        }
        let envBlock = "\n# MayaFlux (installed via Homebrew)\nsource \"\(prefixOutput)/env.sh\"\nexport PATH=\"$HOME/.local/bin:$PATH\"\n"
        handle.seekToEndOfFile()
        handle.write(Data(envBlock.utf8))
        handle.closeFile()

        log("✅ Environment configured")
        step = .done
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
                .disabled({
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
            case .channelSelect:  channelSelectBody
            case .passwordPrompt: passwordPromptBody
            case .running:        runningBody
            case .done:           doneBody
            case .failed(let m):  failedBody(m)
            }
        }
        .frame(width: 580, height: 480)
        .alert("Conflicting Installation", isPresented: $showConflictAlert) {
            Button("Cancel", role: .cancel) { }
            Button("Remove \(conflictingFormula) and continue", role: .destructive) {
                Task { await state.startInstall() }
            }
        } message: {
            Text("You have \(conflictingFormula) installed. Both versions cannot coexist.\n\nRemove it and install \(state.formula)?")
        }
    }

    // ---- Channel Select ----

    var channelSelectBody: some View {
        VStack(spacing: 24) {
            Spacer()
            Text("Select Release Channel")
                .font(.title2).fontWeight(.semibold)

            VStack(spacing: 12) {
                channelButton(label: "Stable", description: "Recommended for most users",
                              icon: "checkmark.seal", selected: state.formula == "mayaflux") {
                    state.formula = "mayaflux"
                }
                channelButton(label: "Development", description: "Latest features, may be unstable",
                              icon: "bolt", selected: state.formula == "mayaflux-dev") {
                    state.formula = "mayaflux-dev"
                }
            }
            .padding(.horizontal, 40)

            Spacer()

            Button("Continue") { advanceFromChannelSelect() }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
                .padding(.bottom, 24)
        }
    }

    func channelButton(label: String, description: String, icon: String, selected: Bool, action: @escaping () -> Void) -> some View {
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
            .overlay(RoundedRectangle(cornerRadius: 10)
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
            Text("Homebrew is not installed. Installing it requires your password once to set up its directory.\n\nYour password is not stored.")
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
            Image(systemName: "checkmark.circle.fill").font(.system(size: 64)).foregroundColor(.green)
            Text("Installation Complete!").font(.title).fontWeight(.bold)
            Text("Restart your terminal for environment changes to take effect.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
                .padding(.horizontal, 40)
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

    private var weavePath: String {
        let localBin = "\(NSHomeDirectory())/.local/bin/weave"
        if FileManager.default.fileExists(atPath: localBin) { return localBin }
        return "\(Bundle.main.resourcePath ?? "")/project_creator.sh"
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

        guard FileManager.default.fileExists(atPath: weavePath) else {
            alertMessage = "Weave CLI not found at:\n\(weavePath)\n\nPlease run Install MayaFlux first."
            showAlert = true
            isCreating = false
            return
        }

        var args = ["new", projectName, projectLocation]
        if withLila { args.append("--with-lila") }

        let task = Process()
        task.executableURL = URL(fileURLWithPath: weavePath)
        task.arguments = args

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
