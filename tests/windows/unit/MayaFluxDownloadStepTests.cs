using System.Net;
using NUnit.Framework;
using Weave;
using Weave.Shared.Models;
using Weave.UI.Pages;

namespace Weave.Windows.UnitTests;

[TestFixture]
public sealed class MayaFluxDownloadStepTests
{
    [Test]
    public async Task StableDownloadSelectsTheX64ArchiveAndEnablesProceedingAfterVerification()
    {
        var host = new FakeMayaFluxDownloadHost();
        var result = await RunAsync(host);

        Assert.That(result.Passed, Is.True);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false, true }));
        Assert.That(host.Requests.Select(request => request.Url), Is.EqualTo(new[]
        {
            "https://api.github.com/repos/MayaFlux/MayaFlux/releases/latest", FakeMayaFluxDownloadHost.AssetUrl
        }));
        Assert.That(host.Requests.All(request => request.UserAgent == "Weave-Installer"), Is.True);
        var extraction = host.Extractions.Single();
        Assert.That(Path.GetDirectoryName(extraction.Archive), Is.EqualTo(FakeMayaFluxDownloadHost.TemporaryRoot));
        Assert.That(Path.GetExtension(extraction.Archive), Is.EqualTo(".7z"));
        Assert.That(extraction.Destination, Does.StartWith(FakeMayaFluxDownloadHost.InstallRoot + ".weave-staging-"));
        Assert.That(host.Verifications, Is.EqualTo(new[] { extraction.Destination }));
        Assert.That(host.Moves.Single().Destination, Is.EqualTo(FakeMayaFluxDownloadHost.InstallRoot));
        Assert.That(result.Progress.Last(), Is.EqualTo(100));
    }

    [Test]
    public async Task DevelopmentDownloadIgnoresStableReleasesAndSelectsTheDevAsset()
    {
        var devUrl = FakeMayaFluxDownloadHost.AssetUrl.Replace("v0.5.0/", "v0.5.0-dev/").Replace("0.5.0-windows", "0.5.0-dev-windows");
        var host = new FakeMayaFluxDownloadHost
        {
            Metadata = "[" + FakeMayaFluxDownloadHost.Release() + "," +
                FakeMayaFluxDownloadHost.Release("v0.5.0-dev", "MayaFlux-0.5.0-dev-windows-x64.7z", devUrl, prerelease: true) + "]"
        };
        var result = await RunAsync(host, ReleaseType.Development);

        Assert.That(result.Passed, Is.True);
        Assert.That(host.Requests[0].Url, Is.EqualTo("https://api.github.com/repos/MayaFlux/MayaFlux/releases"));
        Assert.That(host.Requests[1].Url, Is.EqualTo(devUrl));
    }

    [TestCase("not JSON")]
    [TestCase("[]")]
    [TestCase("{\"tag_name\":null,\"assets\":[]}")]
    [TestCase("{\"tag_name\":\"v0.5.0\",\"assets\":null}")]
    public async Task MalformedReleaseMetadataBlocksProceedingWithoutDeletingAnInstallation(string json)
    {
        var host = new FakeMayaFluxDownloadHost { Metadata = json };
        host.ExistingInstallation();
        var result = await RunAsync(host);

        result.AssertBlocked();
        Assert.That(host.DeletedDirectories, Is.Empty);
        Assert.That(host.WrittenArchives, Is.Empty);
        Assert.That(host.Requests, Has.Count.EqualTo(1));
    }

    [TestCase("[]")]
    public async Task MissingDevelopmentReleaseProducesAClearFailure(string json)
    {
        var result = await RunAsync(new FakeMayaFluxDownloadHost { Metadata = json }, ReleaseType.Development);
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("No releases found")), Is.True);
    }

    [TestCase("MayaFlux-0.5.0-linux-x64.tar.gz")]
    [TestCase("MayaFlux-0.5.0-windows-arm64.7z")]
    [TestCase("MayaFlux-0.5.0-dev-windows-x64.7z")]
    public async Task WrongPlatformOrChannelAssetCannotBeDownloadedAsStable(string name)
    {
        var host = new FakeMayaFluxDownloadHost { Metadata = FakeMayaFluxDownloadHost.Release(name: name) };
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("No matching Windows asset")), Is.True);
        Assert.That(host.Requests, Has.Count.EqualTo(1));
    }

    [TestCase(true, false)]
    [TestCase(false, true)]
    public async Task DraftOrPrereleaseCannotBeInstalledThroughTheStableChannel(bool draft, bool prerelease)
    {
        var host = new FakeMayaFluxDownloadHost { Metadata = FakeMayaFluxDownloadHost.Release(draft: draft, prerelease: prerelease) };
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Requests, Has.Count.EqualTo(1));
    }

    [TestCase("http://github.com/MayaFlux/MayaFlux/releases/download/v0.5.0/package.7z")]
    [TestCase("https://attacker.example/package.7z")]
    [TestCase("https://github.com/SomeoneElse/OtherProject/releases/download/v1/package.7z")]
    public async Task UntrustedAssetLocationIsRejectedBeforeAnArchiveRequest(string url)
    {
        var host = new FakeMayaFluxDownloadHost { Metadata = FakeMayaFluxDownloadHost.Release(url: url) };
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Requests, Has.Count.EqualTo(1));
        Assert.That(host.WrittenArchives, Is.Empty);
    }

    [TestCase(HttpStatusCode.InternalServerError)]
    public async Task MetadataHttpFailurePreservesTheExistingInstallation(HttpStatusCode status)
    {
        var host = new FakeMayaFluxDownloadHost { MetadataStatus = status };
        host.ExistingInstallation();
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.DeletedDirectories, Is.Empty);
        Assert.That(host.Extractions, Is.Empty);
    }

    [TestCase(HttpStatusCode.InternalServerError)]
    public async Task ArchiveHttpFailurePreservesTheExistingInstallation(HttpStatusCode status)
    {
        var host = new FakeMayaFluxDownloadHost { ArchiveStatus = status };
        host.ExistingInstallation();
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.DeletedDirectories, Is.Empty);
        Assert.That(host.Files.TryGetValue(Path.Combine(FakeMayaFluxDownloadHost.InstallRoot, "keep.txt"), out var existing), Is.True);
        Assert.That(existing, Is.EqualTo("existing installation"u8.ToArray()));
        Assert.That(host.Extractions, Is.Empty);
    }

    [Test]
    public async Task ArchiveNetworkTimeoutIsReportedAndPreservesTheExistingInstallation()
    {
        var host = new FakeMayaFluxDownloadHost { ArchiveError = new TaskCanceledException("Archive download timed out") };
        host.ExistingInstallation();
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("timed out")), Is.True);
        Assert.That(host.DeletedDirectories, Is.Empty);
    }

    [Test]
    public async Task InterruptedArchiveCannotBeExtractedOrLeftAsACompleteDownload()
    {
        var host = new FakeMayaFluxDownloadHost { InterruptArchive = true };
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Extractions, Is.Empty);
        Assert.That(host.Files.Keys.Any(path => path.StartsWith(FakeMayaFluxDownloadHost.TemporaryRoot) && path.EndsWith(".7z")), Is.False);
    }

    [TestCase(true)]
    [TestCase(false)]
    public async Task EmptyOrNonArchivePayloadIsRejectedBeforeExtraction(bool empty)
    {
        var host = new FakeMayaFluxDownloadHost { ArchiveBytes = empty ? [] : "HTML error page"u8.ToArray() };
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Extractions, Is.Empty);
    }

    [TestCase(0)]
    [TestCase(100)]
    public async Task IncorrectDeclaredArchiveLengthBlocksExtraction(long length)
    {
        var host = new FakeMayaFluxDownloadHost { DeclaredLength = length };
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Extractions, Is.Empty);
        Assert.That(result.Progress.All(progress => progress is >= 0 and <= 100), Is.True);
    }

    [Test]
    public async Task DownloadWithoutContentLengthDoesNotDivideByZeroOrInventProgress()
    {
        var host = new FakeMayaFluxDownloadHost { UnknownLength = true };
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(result.Progress, Is.EqualTo(new[] { 0 }));
    }

    [Test]
    public async Task ArchiveWriteFailurePreservesTheExistingInstallation()
    {
        var host = new FakeMayaFluxDownloadHost { WriteError = new IOException("Disk full") };
        host.ExistingInstallation();
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(result.Messages.Any(message => message.Contains("Disk full")), Is.True);
        Assert.That(host.DeletedDirectories, Is.Empty);
        Assert.That(host.Extractions, Is.Empty);
    }

    [TestCase(@"C:\")]
    [TestCase("relative directory")]
    [TestCase(@"\\server\share")]
    public async Task UnsafeInstallationDestinationIsRejectedBeforeHostMutation(string path)
    {
        var host = new FakeMayaFluxDownloadHost();
        var result = await RunAsync(host, config: Config(path));
        result.AssertBlocked();
        Assert.That(host.CreatedDirectories, Is.Empty);
        Assert.That(host.Requests, Is.Empty);
        Assert.That(host.DeletedDirectories, Is.Empty);
    }

    [Test]
    public async Task TemporaryDirectoryInsideTheInstallationCannotBeDeletedDuringReplacement()
    {
        var host = new FakeMayaFluxDownloadHost();
        host.ExistingInstallation();
        var config = Config();
        config.TempDirectory = Path.Combine(config.MayaFluxRoot, "download");
        var result = await RunAsync(host, config: config);
        result.AssertBlocked();
        Assert.That(host.DeletedDirectories, Is.Empty);
        Assert.That(host.Requests, Is.Empty);
    }

    [TestCase(@"C:\")]
    [TestCase("relative directory")]
    [TestCase(@"\\server\share")]
    public async Task UnsafeTemporaryDestinationIsRejectedBeforeHostMutation(string path)
    {
        var host = new FakeMayaFluxDownloadHost();
        var config = Config();
        config.TempDirectory = path;
        var result = await RunAsync(host, config: config);
        result.AssertBlocked();
        Assert.That(host.CreatedDirectories, Is.Empty);
        Assert.That(host.Requests, Is.Empty);
    }

    [Test]
    public async Task PreexistingTemporaryArchiveIsNotOverwritten()
    {
        var host = new FakeMayaFluxDownloadHost();
        var path = Path.Combine(FakeMayaFluxDownloadHost.TemporaryRoot, "mayaflux.7z");
        host.Files[path] = "unrelated file"u8.ToArray();
        await RunAsync(host);
        Assert.That(host.Files[path], Is.EqualTo("unrelated file"u8.ToArray()));
    }

    [TestCase(false, true)]
    [TestCase(true, false)]
    public async Task FailedReplacementPreservesThePreviousInstallation(bool extract, bool verify)
    {
        var host = new FakeMayaFluxDownloadHost { ExtractionResult = extract, VerificationResult = verify };
        host.ExistingInstallation();
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Files.ContainsKey(Path.Combine(FakeMayaFluxDownloadHost.InstallRoot, "keep.txt")), Is.True);
        Assert.That(host.Verifications, Has.Count.EqualTo(extract ? 1 : 0));
        Assert.That(host.Files.Keys.Any(path => path.Contains(".weave-staging-") || path.EndsWith(".7z")), Is.False);
    }

    [Test]
    public async Task FailedInstallationMoveRestoresThePreviousDirectory()
    {
        var host = new FakeMayaFluxDownloadHost
        {
            MoveError = (source, destination) => source.Contains(".weave-staging-") ? new IOException("Replacement move denied") : null
        };
        host.ExistingInstallation();
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Files.ContainsKey(Path.Combine(FakeMayaFluxDownloadHost.InstallRoot, "keep.txt")), Is.True);
        Assert.That(result.Messages.Any(message => message.Contains("Replacement move denied")), Is.True);
    }

    [Test]
    public async Task FailedRollbackPreservesAndReportsTheBackupLocation()
    {
        var host = new FakeMayaFluxDownloadHost
        {
            MoveError = (source, destination) => source.Contains(".weave-staging-") || source.Contains(".weave-backup-")
                ? new IOException("Move denied") : null
        };
        host.ExistingInstallation();
        var result = await RunAsync(host);
        result.AssertBlocked();
        var backup = host.Files.Keys.Single(path => path.Contains(".weave-backup-") && path.EndsWith("keep.txt"));
        Assert.That(result.Messages.Any(message => message.Contains(Path.GetDirectoryName(backup)!)), Is.True);
    }

    [Test]
    public async Task BackupCleanupFailureLeavesTheVerifiedReplacementUsable()
    {
        var host = new FakeMayaFluxDownloadHost
        {
            DeleteDirectoryError = path => path.Contains(".weave-backup-") ? new IOException("Backup cleanup denied") : null
        };
        host.ExistingInstallation();
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(host.Files.ContainsKey(Path.Combine(FakeMayaFluxDownloadHost.InstallRoot, "new-installation.txt")), Is.True);
        Assert.That(host.Files.Keys.Any(path => path.Contains(".weave-backup-") && path.EndsWith("keep.txt")), Is.True);
        Assert.That(result.Messages.Any(message => message.StartsWith("[WARN]") && message.Contains("Backup cleanup denied")), Is.True);
    }

    [TestCase(false)]
    [TestCase(true)]
    public async Task ReparsePointAncestorCannotRedirectInstallationOrTemporaryWrites(bool temporary)
    {
        var host = new FakeMayaFluxDownloadHost();
        host.ReparsePoints.Add(temporary ? @"D:\Test User\Temp" : @"D:\Creative Tools");
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Requests, Is.Empty);
        Assert.That(host.CreatedDirectories, Is.Empty);
    }

    [Test]
    public async Task MatchingStableVersionSkipsArchiveAcquisition()
    {
        var host = new FakeMayaFluxDownloadHost();
        host.ExistingInstallation(FakeMayaFluxDownloadHost.Version());
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(result.Readiness, Is.EqualTo(new[] { false, true }));
        Assert.That(host.Requests, Has.Count.EqualTo(1));
        Assert.That(host.DeletedDirectories, Is.Empty);
    }

    [Test]
    public async Task MatchingVersionCannotSkipVerificationOfAnIncompleteInstallation()
    {
        var host = new FakeMayaFluxDownloadHost { VerificationResult = false };
        host.ExistingInstallation(FakeMayaFluxDownloadHost.Version());
        var result = await RunAsync(host);
        result.AssertBlocked();
    }

    [Test]
    public async Task StableChannelDoesNotTreatTheSameNumberedDevelopmentBuildAsStable()
    {
        var host = new FakeMayaFluxDownloadHost();
        host.ExistingInstallation(FakeMayaFluxDownloadHost.Version("0.5.0-dev"));
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(host.Requests, Has.Count.EqualTo(2));
    }

    [TestCase("0.5.0")]
    public async Task NewerNumericVersionDownloadsRatherThanComparingLexically(string local)
    {
        var host = new FakeMayaFluxDownloadHost { Metadata = FakeMayaFluxDownloadHost.Release("v0.10.0",
            "MayaFlux-0.10.0-windows-x64.7z", FakeMayaFluxDownloadHost.AssetUrl.Replace("0.5.0", "0.10.0")) };
        host.ExistingInstallation(FakeMayaFluxDownloadHost.Version(local));
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(host.Requests, Has.Count.EqualTo(2));
    }

    [TestCase("{broken JSON")]
    public async Task MalformedLocalVersionRedownloadsWithAWarning(string version)
    {
        var host = new FakeMayaFluxDownloadHost();
        host.ExistingInstallation(version);
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(result.Messages.Any(message => message.StartsWith("[WARN]")), Is.True);
        Assert.That(host.Requests, Has.Count.EqualTo(2));
    }

    [Test]
    public async Task InvalidLocalVersionCannotSuppressAValidReleaseDownload()
    {
        var host = new FakeMayaFluxDownloadHost();
        host.ExistingInstallation(FakeMayaFluxDownloadHost.Version("zzz-not-a-version"));
        var result = await RunAsync(host);
        Assert.That(result.Passed, Is.True);
        Assert.That(host.Requests, Has.Count.EqualTo(2));
    }

    [TestCase(false)]
    [TestCase(true)]
    public async Task MetadataNetworkFailureIsReportedWithoutArchiveAcquisition(bool timeout)
    {
        var host = new FakeMayaFluxDownloadHost
        {
            MetadataError = timeout ? new TaskCanceledException("Release request timed out") : new HttpRequestException("Release API unavailable")
        };
        var result = await RunAsync(host);
        result.AssertBlocked();
        Assert.That(host.Requests, Has.Count.EqualTo(1));
        Assert.That(host.WrittenArchives, Is.Empty);
    }

    [TestCase("2026-10-12 12:00:01 UTC", false)]
    [TestCase("2026-10-12 12:00:00 UTC", true)]
    [TestCase("2026-10-12 11:59:59 UTC", true)]
    [TestCase("unparseable timestamp", true)]
    [TestCase("2026-10-14 12:00:00 UTC", true)]
    public async Task DevelopmentFreshnessUsesTheUtcThresholdAndRejectsFutureBuildTimes(string buildTime, bool download)
    {
        var host = new FakeMayaFluxDownloadHost
        {
            Metadata = "[" + FakeMayaFluxDownloadHost.Release("v0.5.0-dev", "MayaFlux-0.5.0-dev-windows-x64.7z", prerelease: true) + "]"
        };
        host.ExistingInstallation(FakeMayaFluxDownloadHost.Version(buildTime: buildTime));
        var result = await RunAsync(host, ReleaseType.Development);
        Assert.That(result.Passed, Is.True);
        Assert.That(host.Requests, Has.Count.EqualTo(download ? 2 : 1));
    }

    [Test]
    public async Task RetryFailureRevokesReadinessFromAPreviousSuccessfulDownload()
    {
        var host = new FakeMayaFluxDownloadHost();
        var step = new MayaFluxDownloadStep(ReleaseType.Stable, host);
        var readiness = new List<bool>();
        step.ProceedChanged += readiness.Add;
        Assert.That(await step.DownloadMayaFluxAsync(Config()), Is.True);
        host.MetadataStatus = HttpStatusCode.InternalServerError;
        Assert.That(await step.DownloadMayaFluxAsync(Config()), Is.False);
        Assert.That(readiness.Last(), Is.False);
    }

    private static InstallationConfig Config(string root = FakeMayaFluxDownloadHost.InstallRoot) => new()
    {
        MayaFluxRoot = root, TempDirectory = FakeMayaFluxDownloadHost.TemporaryRoot
    };
    private static async Task<StepObservation> RunAsync(FakeMayaFluxDownloadHost host, ReleaseType channel = ReleaseType.Stable, InstallationConfig? config = null)
    {
        var step = new MayaFluxDownloadStep(channel, host);
        var observation = new StepObservation();
        step.MessageLogged += observation.Messages.Add;
        step.ProceedChanged += observation.Readiness.Add;
        step.ProgressChanged += observation.Progress.Add;
        observation.Passed = await step.DownloadMayaFluxAsync(config ?? Config());
        return observation;
    }
}
