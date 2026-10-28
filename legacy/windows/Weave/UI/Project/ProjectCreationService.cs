using Weave.Utils;
using System.Text.RegularExpressions;

namespace Weave.UI.Project;

internal class ProjectCreationHost : WindowsInstallationHost
{
    internal virtual string ReadAllText(string path) => File.ReadAllText(path);
    internal virtual void WriteAllText(string path, string text) => File.WriteAllText(path, text);
    internal virtual void CopyFile(string source, string destination) => File.Copy(source, destination);
    internal virtual string[] GetFiles(string path) => Directory.GetFiles(path);
    internal virtual void DeleteDirectory(string path) => Directory.Delete(path, true);
}

internal sealed class ProjectCreationService(string templatesDir, ProjectCreationHost host, Action<string> report)
{
    private readonly Dictionary<string, string> templates = new(StringComparer.OrdinalIgnoreCase);
    private void Log(string message) => report(message);
    internal string Create(string projectName, string projectPath, bool withLila = false, bool withVscode = true, bool copyShaderSources = false)
    {
        if (!Regex.IsMatch(projectName, @"\A[A-Za-z_][A-Za-z0-9_]*\z") ||
            Regex.IsMatch(projectName, @"\A(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])\z", RegexOptions.IgnoreCase))
            throw new ArgumentException("Project name must start with a letter or underscore and contain only letters, digits or underscores; Windows device names are not allowed.");
        if (!WindowsInstallationPaths.IsSafeEnvironmentPath(projectPath) || !host.DirectoryExists(projectPath))
            throw new ArgumentException($"Project destination must be an existing absolute directory: {projectPath}");
        var projectDir = Path.Combine(Path.GetFullPath(projectPath), projectName);
        if (host.DirectoryExists(projectDir) || host.FileExists(projectDir))
            throw new IOException($"Project destination already exists: {projectDir}");
        WindowsInstallationPaths.EnsureNoReparseAncestors(projectDir, host);
        WindowsInstallationPaths.EnsureNoReparseAncestors(templatesDir, host);
        templates.Clear();
        foreach (var relative in new[] { "CMakeLists.txt", "CMakePresets.json", "main.cpp", "user_project.hpp", "validate.ps1", "run-report.ps1", ".gitignore", @"cmake\mayaflux.cmake", @"cmake\shaders.cmake", @"cmake\build_community.cmake" }) LoadTemplate(relative);
        if (withVscode)
        {
            foreach (var relative in new[] { @"vscode\settings.json", @"vscode\tasks.json", @"vscode\launch.json" })
            {
                try { LoadTemplate(relative); }
                catch (IOException error) { Log($"  WARNING: Optional VS Code file unavailable: {relative}. {error.Message} Configure this file manually when needed."); }
                catch (UnauthorizedAccessException error) { Log($"  WARNING: Optional VS Code file unavailable: {relative}. {error.Message} Configure this file manually when needed."); }
            }
        }
        var mayaFluxRoot = host.GetEnvironmentVariable("MAYAFLUX_ROOT", EnvironmentVariableTarget.Process) ?? "C:\\MayaFlux";
        var installedShaderDir = Path.Combine(mayaFluxRoot, "share", "MayaFlux", "src", "shaders");
        var installedShaders = copyShaderSources ? GetShaderSourceFiles(installedShaderDir) : Array.Empty<string>();
        var created = false;
        try
        {
            host.CreateDirectory(projectDir);
            created = true;
            return CreateFiles(projectName, projectDir, withLila, withVscode, mayaFluxRoot, installedShaderDir, installedShaders);
        }
        catch (Exception error)
        {
            if (created && host.DirectoryExists(projectDir))
            {
                try
                {
                    WindowsInstallationPaths.EnsureNoReparseAncestors(projectDir, host);
                    host.DeleteDirectory(projectDir);
                }
                catch (Exception cleanup) { throw new AggregateException($"Project creation failed: {error.Message}. Incomplete project could not be removed: {projectDir}. {cleanup.Message}", error, cleanup); }
            }
            throw;
        }
    }

    private string CreateFiles(string projectName, string projectDir, bool withLila, bool withVscode, string mayaFluxRoot, string installedShaderDir, string[] installedShaders)
    {
        var srcDir = Path.Combine(projectDir, "src");
        var cmakeDir = Path.Combine(projectDir, "cmake");
        var vsCodeDir = Path.Combine(projectDir, ".vscode");
        var validator = LoadTemplate("validate.ps1");

        Log($"Creating project structure at: {projectDir}");

        host.CreateDirectory(srcDir);
        Log("  Created src/");

        host.CreateDirectory(cmakeDir);
        Log("  Created cmake/");

        if (withVscode)
        {
            host.CreateDirectory(vsCodeDir);
            Log("  Created .vscode/");
        }

        Log($"  Using MAYAFLUX_ROOT: {mayaFluxRoot}");
        host.WriteAllText(Path.Combine(projectDir, "validate.ps1"), validator);
        Log("  Copied validate.ps1 (run manually for diagnostics)");
        host.WriteAllText(Path.Combine(projectDir, "run-report.ps1"), LoadTemplate("run-report.ps1").Replace("@PROJECT_NAME@", projectName));
        foreach (var name in new[] { "validate", "run-report" })
        {
            var launcher = "@echo off\r\nsetlocal DisableDelayedExpansion\r\n" +
                $"\"%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\" -NoProfile -ExecutionPolicy Bypass -File \"%~dp0{name}.ps1\" %*\r\n" +
                "exit /b %ERRORLEVEL%\r\n";
            host.WriteAllText(Path.Combine(projectDir, name + ".bat"), launcher);
        }
        Log("  Generated run-report.ps1, validate.bat and run-report.bat (run manually for diagnostics)");
        Log("Generating CMakeLists.txt");

        var cmakelists = LoadTemplate("CMakeLists.txt")
            .Replace("@PROJECT_NAME@", projectName);

        host.WriteAllText(Path.Combine(projectDir, "CMakeLists.txt"), cmakelists);
        Log("  Generated CMakeLists.txt");
        Log("Generating cmake modules");

        string lilaLinkBlock;
        string lilaDebuggerPath;
        string lilaDllCopy;

        if (withLila)
        {
            lilaLinkBlock = "target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::MayaFluxHost)";
            lilaDebuggerPath = "$<TARGET_FILE_DIR:MayaFlux::MayaFluxHost>;";
            lilaDllCopy =
                "if(EXISTS \"$ENV{MAYAFLUX_ROOT}/bin/MayaFluxHost.dll\")\r\n" +
                "        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD\r\n" +
                "            COMMAND ${CMAKE_COMMAND} -E copy_if_different\r\n" +
                "                \"$ENV{MAYAFLUX_ROOT}/bin/MayaFluxHost.dll\"\r\n" +
                "                $<TARGET_FILE_DIR:${PROJECT_NAME}>\r\n" +
                "        )\r\n" +
                "    endif()";
        }
        else
        {
            lilaLinkBlock = "";
            lilaDebuggerPath = "";
            lilaDllCopy = "";
        }

        var mayafluxCmake = LoadTemplate(Path.Combine("cmake", "mayaflux.cmake"))
            .Replace("@LILA_LINK_BLOCK@", lilaLinkBlock)
            .Replace("@LILA_DEBUGGER_PATH@", lilaDebuggerPath)
            .Replace("@LILA_DLL_COPY@", lilaDllCopy);

        host.WriteAllText(Path.Combine(cmakeDir, "mayaflux.cmake"), mayafluxCmake);
        Log("  Generated cmake/mayaflux.cmake");

        var shadersCmakeSrc = Path.Combine(templatesDir, "cmake", "shaders.cmake");
        if (host.FileExists(shadersCmakeSrc))
        {
            host.CopyFile(shadersCmakeSrc, Path.Combine(cmakeDir, "shaders.cmake"));
            Log("  Copied cmake/shaders.cmake");
        }
        else
        {
            throw new FileNotFoundException("Required template missing: cmake/shaders.cmake");
        }

        var buildCommunitySrc = Path.Combine(templatesDir, "cmake", "build_community.cmake");
        if (host.FileExists(buildCommunitySrc))
        {
            host.CopyFile(buildCommunitySrc, Path.Combine(cmakeDir, "build_community.cmake"));
            Log("  Copied cmake/build_community.cmake");
        }
        else
        {
            throw new FileNotFoundException("Required template missing: cmake/build_community.cmake");
        }
        Log("Generating source files");

        host.WriteAllText(Path.Combine(srcDir, "main.cpp"), LoadTemplate("main.cpp"));
        Log("  Generated main.cpp");

        host.WriteAllText(Path.Combine(srcDir, "user_project.hpp"), LoadTemplate("user_project.hpp"));
        Log("  Generated user_project.hpp");
        var dataShadersDir = Path.Combine(projectDir, "data", "shaders");
        host.CreateDirectory(dataShadersDir);
        Log("  Created data/shaders/");

        foreach (var shader in installedShaders)
        {
            var relative = Path.GetRelativePath(installedShaderDir, shader);
            var destination = Path.Combine(dataShadersDir, relative);
            if (host.FileExists(destination))
            {
                Log($"  Preserved project shader: {relative}");
                continue;
            }
            host.CreateDirectory(Path.GetDirectoryName(destination)!);
            host.CopyFile(shader, destination);
            Log($"  Copied MayaFlux shader source: {relative}");
        }
        host.WriteAllText(Path.Combine(projectDir, "community.cmake"), "");
        Log("  Created community.cmake");
        var gitignoreSrc = Path.Combine(templatesDir, ".gitignore");
        if (host.FileExists(gitignoreSrc))
        {
            host.CopyFile(gitignoreSrc, Path.Combine(projectDir, ".gitignore"));
            Log("  Copied .gitignore");
        }
        else
        {
            throw new FileNotFoundException("Required template missing: .gitignore");
        }

        var cmakePresetsSrc = Path.Combine(templatesDir, "CMakePresets.json");
        if (!host.FileExists(cmakePresetsSrc))
            throw new FileNotFoundException("Required template missing: CMakePresets.json");
        host.CopyFile(cmakePresetsSrc, Path.Combine(projectDir, "CMakePresets.json"));
        Log("  Copied CMakePresets.json");

        if (withVscode)
        {
            Log("Generating VS Code configuration");
            foreach (var vscodeFile in new[] { "settings.json", "tasks.json", "launch.json" })
            {
                if (templates.TryGetValue(Path.Combine("vscode", vscodeFile), out var template))
                {
                    host.WriteAllText(Path.Combine(vsCodeDir, vscodeFile), template.Replace("@PROJECT_NAME@", projectName));
                    Log($"  Generated .vscode/{vscodeFile}");
                }
            }
        }
        Log("Generating README.md");
        var readme =
            $"# {projectName}\r\n\r\n" +
            "A MayaFlux multimedia DSP project.\r\n\r\n" +
            "## Diagnostics\r\n\r\n" +
            "When you need diagnostics, run this manually from the project folder:\r\n\r\n" +
            "```powershell\r\n" +
            ".\\validate.bat\r\n" +
            "```\r\n\r\n" +
            "It checks the project, MayaFlux installation, tools and terminal settings without building or changing anything. Copy the report when requesting help.\r\n\r\n" +
            "After building, collect logs for a bug report:\r\n\r\n" +
            "```powershell\r\n.\\run-report.bat\r\n```\r\n\r\n" +
            "Reproduce the problem, then close the application. Attach `mayaflux_run.txt` and `data/log/mayaflux_report.log` to your issue. Previous logs are preserved with a `.previous-` suffix.\r\n\r\n" +
            "For multiple build configurations, use `run-report.bat -Configuration Debug`. For a custom output folder, use `run-report.bat -Executable \"path\\to\\application.exe\"`. For a custom installation, add `-MayaFluxRoot \"C:\\Your MayaFlux Folder\"`.\r\n\r\n" +
            "Both `.ps1` scripts are included and can also be run directly when your PowerShell execution policy permits it. The batch launchers preserve the calling terminal's environment.\r\n\r\n" +
            "## Building\r\n\r\n" +
            "```powershell\r\n" +
            "cmake --preset release\r\n" +
            "cmake --build --preset release --parallel\r\n" +
            "```\r\n\r\n" +
            "## Running\r\n\r\n" +
            $"```powershell\r\n.\\build\\{projectName}.exe\r\n```\r\n\r\n" +
            "## Editing\r\n\r\n" +
            "Open in VS Code:\r\n```bash\r\ncode .\r\n```\r\n\r\n" +
            "Edit your code in `src/user_project.hpp`:\r\n" +
            "- `settings()`: Configure sample rate, buffer size, graphics\r\n" +
            "- `compose()`: Create your nodes, buffers, and processing chains\r\n\r\n" +
            "## Documentation\r\n\r\n" +
            "See [MayaFlux Documentation](https://github.com/MayaFlux/MayaFlux)\r\n";

        host.WriteAllText(Path.Combine(projectDir, "README.md"), readme);
        Log("  Generated README.md");
        return projectDir;
    }

    private string[] GetShaderSourceFiles(string directory)
    {
        if (!host.DirectoryExists(directory))
            throw new DirectoryNotFoundException($"MayaFlux shader sources were not found at: {directory}. Update your MayaFlux installation or turn off Copy MayaFlux shader sources.");
        WindowsInstallationPaths.EnsureNoReparseAncestors(directory, host);
        var files = new List<string>();
        var extensions = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            ".glsl", ".vert", ".frag", ".comp", ".geom", ".tesc", ".tese", ".mesh", ".task",
            ".rgen", ".rchit", ".rahit", ".rmiss", ".rint", ".rcall"
        };
        var pending = new Stack<string>();
        pending.Push(directory);
        while (pending.Count > 0)
        {
            foreach (var entry in host.GetFileSystemEntries(pending.Pop()))
            {
                var attributes = host.GetAttributes(entry);
                if ((attributes & FileAttributes.ReparsePoint) != 0)
                    throw new IOException($"MayaFlux shader sources contain a linked path: {entry}");
                if ((attributes & FileAttributes.Directory) != 0) pending.Push(entry);
                else if (extensions.Contains(Path.GetExtension(entry))) files.Add(entry);
            }
        }
        if (files.Count == 0)
            throw new IOException($"MayaFlux shader sources are empty at: {directory}. Update your MayaFlux installation or turn off Copy MayaFlux shader sources.");
        return files.ToArray();
    }

    private string LoadTemplate(string templateName)
    {
        if (templates.TryGetValue(templateName, out var cached)) return cached;
        var path = Path.Combine(templatesDir, templateName);
        WindowsInstallationPaths.EnsureNoReparseAncestors(path, host);
        if (!host.FileExists(path))
        {
            throw new FileNotFoundException($"Required template not found: {templateName} at {path}. Use Weave installation to reinstall its templates, then try again.");
        }

        var content = host.ReadAllText(path);
        if (string.IsNullOrWhiteSpace(content)) throw new IOException($"Required template is empty: {templateName}. Use Weave installation to reinstall its templates, then try again.");
        foreach (Match marker in Regex.Matches(content, @"@[A-Z][A-Z0-9_]*@"))
        {
            var valid = marker.Value == "@PROJECT_NAME@" && (templateName == "CMakeLists.txt" || templateName == "run-report.ps1" || templateName.StartsWith("vscode")) ||
                templateName == @"cmake\mayaflux.cmake" && marker.Value is "@LILA_LINK_BLOCK@" or "@LILA_DEBUGGER_PATH@" or "@LILA_DLL_COPY@";
            if (!valid) throw new IOException($"Unexpected template value {marker.Value} in {templateName}");
        }
        templates[templateName] = content;
        return content;
    }

}

