using Newtonsoft.Json;
using Weave.Shared;
using Weave.Shared.Models;

namespace Weave.Utils;

public class GithubApi
{
    private readonly HttpClient httpClient;
    private readonly Logger logger;

    public GithubApi(Logger logger)
    {
        this.logger = logger;
        httpClient = new HttpClient();
        httpClient.DefaultRequestHeaders.Add("User-Agent", "Weave-Installer");
    }

    public async Task<GitHubRelease?> GetLatestReleaseAsync()
    {
        try
        {
            logger.Log($"Fetching latest release from GitHub API...");
            
            var response = await httpClient.GetAsync(WeaveConstants.GITHUB_API_URL);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            var release = JsonConvert.DeserializeObject<GitHubRelease>(json);

            if (release == null)
            {
                logger.Log("[ERROR] Failed to parse GitHub release JSON");
                return null;
            }

            logger.Log($"[OK] Latest release: {release.TagName}");
            return release;
        }
        catch (Exception ex)
        {
            logger.Log($"[ERROR] Failed to fetch release: {ex.Message}");
            return null;
        }
    }
}
