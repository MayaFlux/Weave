using Weave.Shared.Models;

namespace Weave.Modes;

public interface IInstallationStep
{
    Panel CreateUI(InstallationConfig config, Action<string> logCallback, Action nextCallback, InstallationMode parent);
}