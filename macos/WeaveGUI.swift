import SwiftUI
import AppKit

@main
struct WeaveApp: App {
    var body: some Scene {
        WindowGroup {
            WeaveProjectCreatorView()
        }
    }
}

struct WeaveProjectCreatorView: View {
    @State private var projectName: String = "MyFirstProject"
    @State private var projectLocation: String = NSHomeDirectory() + "/Projects"
    @State private var withLila: Bool = false
    @State private var showAlert: Bool = false
    @State private var alertMessage: String = ""
    @State private var isCreating: Bool = false
    
    var body: some View {
        VStack(spacing: 20) {
            // Header
            HStack {
                Image(systemName: "wand.and.stars")
                    .font(.system(size: 40))
                    .foregroundColor(.blue)
                VStack(alignment: .leading) {
                    Text("Weave")
                        .font(.largeTitle)
                        .fontWeight(.bold)
                    Text("MayaFlux Project Creator")
                        .font(.subheadline)
                        .foregroundColor(.gray)
                }
            }
            .padding(.top, 10)
            
            Divider()
            
            // Project Name
            VStack(alignment: .leading, spacing: 8) {
                Text("Project Name:")
                    .font(.headline)
                TextField("MyFirstProject", text: $projectName)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }
            
            // Project Location
            VStack(alignment: .leading, spacing: 8) {
                Text("Project Location:")
                    .font(.headline)
                HStack {
                    TextField("/path/to/location", text: $projectLocation)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                    Button("Browse...") {
                        selectFolder()
                    }
                }
            }
            
            // Options
            Toggle("Enable Lila (Live Coding)", isOn: $withLila)
                .toggleStyle(CheckboxToggleStyle())
            
            Divider()
            
            // Preview
            VStack(alignment: .leading, spacing: 4) {
                Text("Project will be created at:")
                    .font(.caption)
                    .foregroundColor(.gray)
                Text("\(projectLocation)/\(projectName)")
                    .font(.system(.body, design: .monospaced))
                    .foregroundColor(.blue)
            }
            .padding()
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
            
            Spacer()
            
            // Action Buttons
            HStack(spacing: 16) {
                Button("Skip") {
                    NSApplication.shared.terminate(nil)
                }
                .keyboardShortcut(.cancelAction)
                
                Button(isCreating ? "Creating..." : "Create Project") {
                    createProject()
                }
                .disabled(isCreating || projectName.isEmpty)
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(30)
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
        
        // Find weave command
        let possiblePaths = [
            "\(NSHomeDirectory())/.local/bin/weave",
            "/usr/local/bin/weave", 
            "/Library/Weave/project_creator.sh"
        ]
        
        guard let weaveCommand = possiblePaths.first(where: { FileManager.default.fileExists(atPath: $0) }) else {
            alertMessage = "Error: Weave CLI not found.\nExpected locations:\n" + possiblePaths.joined(separator: "\n")
            showAlert = true
            isCreating = false
            return
        }
        
        // Build command arguments
        var args = ["new", projectName, projectLocation]
        if withLila {
            args.append("--with-lila")
        }
        
        // Execute weave command
        let task = Process()
        task.executableURL = URL(fileURLWithPath: weaveCommand)
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
            alertMessage = "Error executing weave: \(error.localizedDescription)"
        }
        
        showAlert = true
        isCreating = false
    }
}

struct CheckboxToggleStyle: ToggleStyle {
    func makeBody(configuration: Configuration) -> some View {
        HStack {
            Image(systemName: configuration.isOn ? "checkmark.square.fill" : "square")
                .foregroundColor(configuration.isOn ? .accentColor : .gray)
                .onTapGesture {
                    configuration.isOn.toggle()
                }
            configuration.label
        }
    }
}
