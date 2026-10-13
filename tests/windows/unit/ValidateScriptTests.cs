using System.Diagnostics;
using NUnit.Framework;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class ValidateScriptTests
{
    [TestCase("healthy", 0)]
    [TestCase("bad-version", 1)]
    [TestCase("missing-ffmpeg-library", 1)]
    [TestCase("stale-shell", 1)]
    [TestCase("old-cmake", 1)]
    [TestCase("tool-timeout", 1)]
    [TestCase("wrong-banner", 1)]
    [TestCase("missing-crt", 1)]
    [TestCase("missing-sdk", 1)]
    [TestCase("denied-read", 1)]
    public async Task PackagedValidatorReportsInstallationProblemsWithoutChangingTheHost(string scenario, int exitCode)
    {
        var (report, errors, actualExitCode) = await RunAsync(scenario);
        Assert.Multiple(() =>
        {
            Assert.That(errors, Is.Empty);
            Assert.That(actualExitCode, Is.EqualTo(exitCode), report);
            Assert.That(report, Does.Contain("MayaFlux installation check"));
            Assert.That(report, Does.Contain("Nothing was changed"));
            Assert.That(report, Does.Contain("Copy this report"));
            if (exitCode != 0) Assert.That(report, Does.Contain("[FIX]"));
            else Assert.That(report, Does.Contain("No problems found"));
        });
    }

    [TestCase("project-healthy", 0, "release preset pair")]
    [TestCase("project-missing-file", 1, "A project build file is missing")]
    [TestCase("project-bad-presets", 1, "Project presets could not be checked")]
    [TestCase("project-unresolved", 1, "template substitution is unfinished")]
    [TestCase("project-missing-module", 1, "listed but its CMake file is missing")]
    [TestCase("project-unsafe-module", 1, "Unsafe or unsupported community module entry")]
    [TestCase("project-preset-probe-fails", 1, "CMake could not read the project presets")]
    [TestCase("project-shader-pending", 0, "shader outputs are missing or older")]
    public async Task ManualProjectDiagnosticsReportProblemsWithoutBuildingOrChangingFiles(string scenario, int exitCode, string message)
    {
        var (report, errors, actualExitCode) = await RunAsync(scenario);
        Assert.Multiple(() =>
        {
            Assert.That(errors, Is.Empty);
            Assert.That(actualExitCode, Is.EqualTo(exitCode), report);
            Assert.That(report, Does.Contain("MayaFlux project diagnostics").And.Contain(message));
            Assert.That(report, Does.Contain("Nothing was changed"));
            Assert.That(report, Does.Contain("FIXTURE_UNCHANGED"));
        });
    }

    private static async Task<(string Report, string Errors, int ExitCode)> RunAsync(string scenario)
    {
        using var directory = new TestDirectory();
        using var resource = typeof(ResourceExtractor).Assembly.GetManifestResourceStream("Weave.Resources.templates.validate.ps1");
        Assert.That(resource, Is.Not.Null, "validate.ps1 must be embedded in Weave");
        var script = Path.Combine(directory.PathName, "validate.ps1");
        using (var file = File.Create(script)) resource!.CopyTo(file);
        var wrapper = Path.Combine(directory.PathName, "run.ps1");
        File.WriteAllText(wrapper, "$fixtureRoot = " + Quote(directory.PathName) + "\r\n$scenario = " + Quote(scenario) + "\r\n. " + Quote(script) + "\r\n" + Fixture);
        var request = new ProcessStartInfo("powershell.exe") { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
        foreach (var argument in new[] { "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", wrapper }) request.ArgumentList.Add(argument);
        using var process = Process.Start(request)!;
        var stdout = process.StandardOutput.ReadToEndAsync();
        var stderr = process.StandardError.ReadToEndAsync();
        try { await process.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30)); }
        catch { process.Kill(true); throw; }
        var report = await stdout;
        var errors = await stderr;
        return (report, errors, process.ExitCode);
    }

    private static string Quote(string value) => "'" + value.Replace("'", "''") + "'";

    private const string Fixture = """
        $ErrorActionPreference = 'Continue'
        $root = Join-Path $fixtureRoot 'Maya Flux'
        $llvm = Join-Path $fixtureRoot 'LLVM'
        $sdk = Join-Path $fixtureRoot 'Vulkan'
        $ffmpeg = Join-Path $fixtureRoot 'FFmpeg'
        $kits = Join-Path $fixtureRoot 'Windows Kits'
        $compiler = Join-Path $fixtureRoot 'MSVC\bin\Hostx64\x64\cl.exe'
        $variables = @{
            MAYAFLUX_ROOT = $root; LLVM_ROOT = $llvm; FFMPEG_ROOT = $ffmpeg
            VULKAN_SDK = $sdk; VK_SDK_PATH = $sdk
            LLVM_DIR = (Join-Path $llvm 'lib\cmake\llvm'); Clang_DIR = (Join-Path $llvm 'lib\cmake\clang')
            CPATH = ((Join-Path $root 'include') + ';' + (Join-Path $sdk 'Include'))
            LIBRARY_PATH = (Join-Path $root 'lib')
            PATH = ((Join-Path $root 'bin') + ';' + (Join-Path $llvm 'bin') + ';' + (Join-Path $sdk 'Bin') + ';' + (Join-Path $ffmpeg 'bin'))
        }
        foreach ($path in @($variables.LLVM_DIR, $variables.Clang_DIR, (Join-Path $root 'include'), (Join-Path $sdk 'Include'))) {
            New-Item -ItemType Directory -Path $path -Force | Out-Null
        }
        $files = @(
            (Join-Path $root 'share\MayaFlux\.version'), (Join-Path $root 'lib\MayaFluxLib.lib'),
            (Join-Path $root 'bin\MayaFluxLib.dll'), (Join-Path $root 'lib\cmake\MayaFlux\MayaFluxConfig.cmake'),
            (Join-Path $llvm 'bin\clang.exe'), (Join-Path $sdk 'Bin\glslc.exe'), (Join-Path $ffmpeg 'bin\ffmpeg.exe'),
            $compiler, (Join-Path $kits 'Include\10.0.22621.0\um\Windows.h'),
            (Join-Path $kits 'Include\10.0.22621.0\um\audioclient.h'),
            (Join-Path $kits 'Include\10.0.22621.0\um\mmdeviceapi.h'),
            (Join-Path $kits 'Include\10.0.22621.0\um\commdlg.h'),
            (Join-Path $kits 'Include\10.0.22621.0\ucrt\stdio.h'),
            (Join-Path $kits 'Lib\10.0.22621.0\um\x64\winmm.lib'),
            (Join-Path $kits 'Lib\10.0.22621.0\ucrt\x64\ucrt.lib')
        )
        foreach ($component in @('avcodec','avformat','avutil','swresample','swscale','avdevice')) {
            $files += (Join-Path $ffmpeg "lib\$component.lib"), (Join-Path $ffmpeg "include\lib$component\$component.h"), (Join-Path $ffmpeg "bin\$component-62.dll")
        }
        foreach ($path in $files) {
            New-Item -ItemType Directory -Path (Split-Path $path) -Force | Out-Null
            Set-Content -LiteralPath $path -Value 'fixture'
        }
        $version = '{"version":"0.5.0-dev","build_time":"2026-10-13 12:00:00 UTC"}'
        if ($scenario -eq 'bad-version') { $version = '{bad json' }
        Set-Content -LiteralPath (Join-Path $root 'share\MayaFlux\.version') -Value $version
        if ($scenario -eq 'missing-ffmpeg-library') { Remove-Item -LiteralPath (Join-Path $ffmpeg 'lib\avcodec.lib') }
        if ($scenario -eq 'missing-sdk') { Remove-Item -LiteralPath (Join-Path $kits 'Include\10.0.22621.0\um\Windows.h') }
        function Get-ValidationEnvironment($Name, $Target = 'Process') {
            if ($scenario -eq 'stale-shell' -and $Target -eq 'Process' -and $Name -eq 'MAYAFLUX_ROOT') { return $null }
            return $variables[$Name]
        }
        function Find-ValidationTool($Name, $Candidates = @()) { return (Join-Path $fixtureRoot "$Name.exe") }
        function Invoke-ValidationTool($Path, $Arguments, $TimeoutSeconds = 15) {
            $name = [IO.Path]::GetFileNameWithoutExtension($Path)
            if ($Arguments -contains '--list-presets=all') {
                if ($scenario -eq 'project-preset-probe-fails') { return @{ Code = 1; Text = 'Invalid preset reference' } }
                return @{ Code = 0; Text = 'release debug' }
            }
            $allowed = ($Arguments.Count -eq 1 -and ($Arguments[0] -eq '--version' -or ($name -eq 'ffmpeg' -and $Arguments[0] -eq '-version') -or ($name -eq '7z' -and $Arguments[0] -eq 'i')))
            if (-not $allowed) { throw 'Unexpected tool operation' }
            $output = switch ($name) {
                'cmake' { if ($scenario -eq 'old-cmake') { 'cmake version 3.25.0' } else { 'cmake version 3.31.0' } }
                'git' { 'git version 2.39.5.windows.1' }
                'ninja' { '1.10.2' }
                'clang' { 'clang version 20.1.8' }
                'ffmpeg' { 'ffmpeg version N-123456-gabcdef' }
                'glslc' { if ($scenario -eq 'wrong-banner') { 'unrelated tool' } else { 'shaderc v2023.8 v2025.3-10-gc7e73e8' } }
                '7z' { '7-Zip 24.09 (x64)' }
            }
            if ($scenario -eq 'tool-timeout' -and $name -eq 'git') { return @{ Code = -1; Text = 'The tool did not finish within 15 seconds.' } }
            return @{ Code = 0; Text = $output }
        }
        function Get-ValidationMicrosoftState {
            return @{ Compiler = $compiler; SdkRoot = $kits; RuntimeInstalled = ($scenario -ne 'missing-crt'); RuntimeVersion = 'v14.44.35211.0' }
        }
        if ($scenario -eq 'denied-read') {
            function Test-Path {
                param($LiteralPath, $PathType)
                if ($LiteralPath -like "$ffmpeg*") { Write-Error 'Access to FFmpeg was denied.'; return $false }
                Microsoft.PowerShell.Management\Test-Path @PSBoundParameters
            }
        }
        if ($scenario.StartsWith('project-')) {
            $project = Join-Path $fixtureRoot 'User Project'
            foreach ($relative in @('cmake', 'src\nested', 'data\shaders')) { New-Item -ItemType Directory -Path (Join-Path $project $relative) -Force | Out-Null }
            foreach ($relative in @('CMakeLists.txt', 'cmake\mayaflux.cmake', 'cmake\shaders.cmake', 'cmake\build_community.cmake', 'src\nested\main.cpp')) { Set-Content -LiteralPath (Join-Path $project $relative) -Value 'fixture content' }
            $presets = '{"version":6,"configurePresets":[{"name":"release"},{"name":"debug"}],"buildPresets":[{"name":"release","configurePreset":"release"},{"name":"debug","configurePreset":"debug"}]}'
            if ($scenario -eq 'project-bad-presets') { $presets = '{broken json' }
            Set-Content -LiteralPath (Join-Path $project 'CMakePresets.json') -Value $presets
            Set-Content -LiteralPath (Join-Path $project 'community.cmake') -Value ''
            if ($scenario -eq 'project-missing-file') { Remove-Item -LiteralPath (Join-Path $project 'cmake\shaders.cmake') }
            if ($scenario -eq 'project-unresolved') { Set-Content -LiteralPath (Join-Path $project 'CMakeLists.txt') -Value 'project(@PROJECT_NAME@)' }
            if ($scenario -eq 'project-missing-module') { Set-Content -LiteralPath (Join-Path $project 'community.cmake') -Value 'missing_module' }
            if ($scenario -eq 'project-unsafe-module') { Set-Content -LiteralPath (Join-Path $project 'community.cmake') -Value '../escape' }
            if ($scenario -eq 'project-shader-pending') { Set-Content -LiteralPath (Join-Path $project 'data\shaders\edit.comp') -Value 'GLSL fixture' }
            function Get-ProjectSnapshot { return ((Get-ChildItem -LiteralPath $project -Recurse -File | Sort-Object FullName | ForEach-Object { $_.FullName + ':' + [Convert]::ToBase64String([IO.File]::ReadAllBytes($_.FullName)) }) -join "`n") }
            $before = Get-ProjectSnapshot
            $code = Invoke-InstallValidation -ProjectDirectory $project
            if ($before -cne (Get-ProjectSnapshot)) { throw 'Diagnostics changed project files' }
            Write-Host 'FIXTURE_UNCHANGED'
            exit $code
        }
        exit (Invoke-InstallValidation)
        """;
}
