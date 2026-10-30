# Windows tests

Run on x64 Windows with the .NET 8 SDK and CMake 3.25 or newer available on PATH:

```powershell
dotnet test tests/windows/unit/Weave.Windows.UnitTests.csproj -c Release -p:Platform=x64
```

The NUnit suite covers installation, project creation, community-module updates and community creation. Controlled fixtures exercise filesystem, HTTP, process and GUI boundaries. Native cases use owned temporary directories, Windows PowerShell and WinForms controls. The community preset check runs CMake only to list presets; it does not configure, compile or build a C++ project.

Tests do not install packages, require elevation, contact live registries, initialize real Git repositories, change host environment variables or execute downloaded installers. Git operations use controlled fixtures. Each temporary directory is confined to the test working directory and cleaned by the shared test helper.

The Windows CI build job runs both suites and fails on a test failure. Unit results are uploaded after the unit run; integration results are uploaded after the packaged run, including on failure. The packaged checks run before the distribution upload.

To test the published ZIP:

```powershell
.github/scripts/verify-windows-build.ps1 -ZipPath build/windows/Weave.zip
```

The integration project has no reference to the production projects or source templates. It reads the single-file bundle from the ZIP, checks x64 GUI architecture, the embedded .NET 8 frameworks and statically linked Windows CLR/JIT, required resources, and the flat executable-only distribution. It loads the packaged resource extractor to prepare an isolated MayaFlux fixture, then launches the packaged executable with its own temporary environment, no host .NET root, and a system-only PATH. Native Windows messages drive the GUI off-screen: project creation and retry preserve user content, missing optional Git permits module creation, and invalid project updates remain recoverable.

Successful update registration and repeated updates use the published Shared assembly with its public HTTP seam and an offline registry fixture. This does not exercise real Git acquisition or a successful network update through the executable. There is no supported Windows CLI dispatcher. Installation elevation construction/cancellation remains covered by controlled unit tests; the packaged checks never accept UAC or run an installer. Project operations launch as the current user; only selecting installation requests elevation.

Unsigned artifacts still undergo all packaging and startup checks. If an artifact has an Authenticode signature, the signature must validate; signing credentials are not required. The local unsigned run does not prove the signed branch. These suites do not prove a generated project's C++ configure/build/run flow or MayaFlux ABI compatibility.

The bundle reader follows the [.NET 8 bundle manifest format](https://github.com/dotnet/runtime/blob/v8.0.0/src/installer/managed/Microsoft.NET.HostModel/Bundle/Manifest.cs). Windows CLR/JIT export checks follow the [single-file host export contract](https://github.com/dotnet/runtime/blob/v8.0.0/src/native/corehost/apphost/static/singlefilehost.def).
