using Weave.Utils;

namespace Weave.UI.Pages;

internal class EnvironmentSetupHost : WindowsInstallationHost
{
    internal virtual Logger CreateLogger() => new();
}
