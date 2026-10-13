using NUnit.Framework;

namespace Weave.Windows.UnitTests;

internal sealed class StepObservation
{
    internal bool Passed { get; set; }
    internal List<string> Messages { get; } = [];
    internal List<bool> Readiness { get; } = [];
    internal List<int> Progress { get; } = [];

    internal static async Task<StepObservation> CaptureAsync(Func<Action<string>, Action<bool>, Task<bool>> run)
    {
        var result = new StepObservation();
        result.Passed = await run(result.Messages.Add, result.Readiness.Add);
        return result;
    }

    internal static async Task<StepObservation> CaptureReadinessAsync(Func<Action<string>, Action<bool>, Task> run)
    {
        var result = new StepObservation();
        await run(result.Messages.Add, result.Readiness.Add);
        result.Passed = result.Readiness.LastOrDefault();
        return result;
    }

    internal void AssertBlocked()
    {
        Assert.Multiple(() =>
        {
            Assert.That(Passed, Is.False);
            Assert.That(Readiness, Is.Not.Empty);
            Assert.That(Readiness.LastOrDefault(), Is.False);
            Assert.That(Readiness, Has.None.True);
            Assert.That(Messages.Any(message => message.Contains("ERROR", StringComparison.OrdinalIgnoreCase)), Is.True);
        });
    }
}

internal sealed class TestDirectory : IDisposable
{
    private readonly string parent = Path.GetFullPath(TestContext.CurrentContext.WorkDirectory).TrimEnd('\\') + "\\";
    internal string PathName { get; }
    internal TestDirectory()
    {
        PathName = Path.GetFullPath(Path.Combine(parent, "weave-test-" + Guid.NewGuid().ToString("N")));
        if (!PathName.StartsWith(parent, StringComparison.OrdinalIgnoreCase)) throw new IOException("Test directory escapes its working directory.");
        Directory.CreateDirectory(PathName);
    }
    public void Dispose()
    {
        if (!Path.GetFullPath(PathName).StartsWith(parent, StringComparison.OrdinalIgnoreCase)) throw new IOException("Unsafe test cleanup path.");
        if (Directory.Exists(PathName)) Directory.Delete(PathName, true);
    }
}
