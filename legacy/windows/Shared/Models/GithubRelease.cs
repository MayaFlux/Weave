using System.Text.Json.Serialization;

namespace Weave.Shared.Models;

public class GitHubRelease
{
    [JsonPropertyName("tag_name")]
    public string TagName { get; set; } = "";
    
    [JsonPropertyName("assets")]
    public List<Asset> Assets { get; set; } = new();
    
    public Asset? GetWindowsAsset()
    {
        return Assets.FirstOrDefault(a => a.Name.Contains("windows") && a.Name.EndsWith(".7z"));
    }
}

public class Asset
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";
    
    [JsonPropertyName("browser_download_url")]
    public string DownloadUrl { get; set; } = "";
}
