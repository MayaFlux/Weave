using System.ComponentModel;
using System.Diagnostics;
using System.Security.Principal;

namespace Weave.Utils;

internal static class InstallationElevation
{
    internal const string InstallationArgument = "--installation";

    internal static bool IsAdministrator()
    {
        using var identity = WindowsIdentity.GetCurrent();
        return new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator);
    }

    internal static bool TryLaunch(string executable, Func<ProcessStartInfo, bool> launch, Action<string> report)
    {
        var request = new ProcessStartInfo(executable)
        {
            UseShellExecute = true,
            Verb = "runas",
            WorkingDirectory = Path.GetDirectoryName(executable)!
        };
        request.ArgumentList.Add(InstallationArgument);
        try
        {
            if (launch(request)) return true;
            report("Could not start the MayaFlux installer. You can retry installation or continue with project operations.");
        }
        catch (Win32Exception error) when (error.NativeErrorCode == 1223)
        {
            report("Installation needs administrator permission. No installation was started. You can retry or continue with project operations.");
        }
        catch (Exception error)
        {
            report($"Could not start the MayaFlux installer: {error.Message}. You can retry or continue with project operations.");
        }
        return false;
    }
}
