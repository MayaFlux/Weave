namespace Weave.Shared.Models;

public class InstallationConfig
{
    public string MayaFluxRoot { get; set; } = WeaveConstants.DEFAULT_INSTALL_DIR;
    public string TempDirectory { get; set; } = Path.Combine(Path.GetTempPath(), "Weave-Install");
    public bool SkipDependencies { get; set; }
    public bool SkipDownload { get; set; }
    
    public string TemplatesDirectory => Path.Combine(MayaFluxRoot, WeaveConstants.WEAVE_TEMPLATES_SUBDIR);
    public string ScriptsDirectory => Path.Combine(MayaFluxRoot, WeaveConstants.WEAVE_SCRIPTS_SUBDIR);
    public string LibDirectory => Path.Combine(MayaFluxRoot, "lib");
    public string BinDirectory => Path.Combine(MayaFluxRoot, "bin");
    public bool NeedsReboot { get; set; } = false;
}