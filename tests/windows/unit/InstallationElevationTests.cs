using System.ComponentModel;
using NUnit.Framework;
using Weave.Utils;

namespace Weave.Windows.UnitTests;

[TestFixture]
public class InstallationElevationTests
{
    [Test]
    public void InstallationHandoffUsesTheSameExecutableAndNativeElevation()
    {
        var executable = @"C:\User Tools\Weave\Weave.exe";
        var messages = new List<string>();
        var launched = InstallationElevation.TryLaunch(executable, request =>
        {
            Assert.Multiple(() =>
            {
                Assert.That(request.FileName, Is.EqualTo(executable));
                Assert.That(request.WorkingDirectory, Is.EqualTo(@"C:\User Tools\Weave"));
                Assert.That(request.UseShellExecute, Is.True);
                Assert.That(request.Verb, Is.EqualTo("runas"));
                Assert.That(request.ArgumentList, Is.EqualTo(new[] { "--installation" }));
            });
            return true;
        }, messages.Add);
        Assert.That(launched, Is.True);
        Assert.That(messages, Is.Empty);
    }

    [TestCase(1223, "administrator permission")]
    [TestCase(2, "Could not start")]
    public void FailedOrCancelledElevationReturnsToTheModeChooserWithRecoveryAdvice(int code, string expected)
    {
        var messages = new List<string>();
        Assert.That(InstallationElevation.TryLaunch(@"C:\Tools\Weave.exe", _ => throw new Win32Exception(code), messages.Add), Is.False);
        Assert.That(messages.Single(), Does.Contain(expected).And.Contain("retry").And.Contain("project operations"));
        Assert.That(InstallationElevation.TryLaunch(@"C:\Tools\Weave.exe", _ => false, messages.Add), Is.False);
        Assert.That(messages.Last(), Does.Contain("retry").And.Contain("project operations"));
    }
}
