using Weave.Shared;
using Weave.Shared.Models;

namespace Weave;

public class ModeDetector
{
    public WeaveMode DetectMode()
    {
        string? mayaFluxRoot = Environment.GetEnvironmentVariable(WeaveConstants.ENV_MAYAFLUX_ROOT);

        // Check if MayaFlux is already installed
        if (!string.IsNullOrEmpty(mayaFluxRoot) && IsMayaFluxInstalled(mayaFluxRoot))
        {
            return WeaveMode.ProjectCreation;
        }

        // Check default location
        if (IsMayaFluxInstalled(WeaveConstants.DEFAULT_INSTALL_DIR))
        {
            return WeaveMode.ProjectCreation;
        }

        // First run: need to install
        return WeaveMode.Installation;
    }

    private bool IsMayaFluxInstalled(string mayaFluxRoot)
    {
        var libPath = Path.Combine(mayaFluxRoot, "lib", "MayaFluxLib.lib");
        return File.Exists(libPath);
    }
}