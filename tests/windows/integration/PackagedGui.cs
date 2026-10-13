using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using NUnit.Framework;

namespace Weave.Windows.IntegrationTests;

internal sealed class PackagedGui : IDisposable
{
    private readonly Process process;
    private delegate bool EnumerateWindow(nint window, nint parameter);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumerateWindow callback, nint parameter);
    [DllImport("user32.dll")] private static extern bool EnumChildWindows(nint parent, EnumerateWindow callback, nint parameter);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(nint window, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(nint window, StringBuilder text, int count);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetClassName(nint window, StringBuilder text, int count);
    [DllImport("user32.dll")] private static extern bool SetWindowPos(nint window, nint after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] private static extern bool PostMessage(nint window, uint message, nint wparam, nint lparam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern nint SendMessage(nint window, uint message, nint wparam, string text);
    [DllImport("user32.dll")] private static extern bool IsWindowEnabled(nint window);

    internal PackagedGui(PublishedBundle bundle, string installation)
    {
        var start = new ProcessStartInfo(bundle.Executable)
        {
            UseShellExecute = false,
            WorkingDirectory = bundle.Root,
            WindowStyle = ProcessWindowStyle.Hidden
        };
        start.Environment["MAYAFLUX_ROOT"] = installation;
        start.Environment["PATH"] = Environment.GetFolderPath(Environment.SpecialFolder.System);
        start.Environment["DOTNET_ROOT"] = Path.Combine(bundle.Root, "No host runtime");
        start.Environment["DOTNET_MULTILEVEL_LOOKUP"] = "0";
        start.Environment["DOTNET_BUNDLE_EXTRACT_BASE_DIR"] = Path.Combine(bundle.Root, "runtime extraction");
        var temporary = Path.Combine(bundle.Root, "temporary");
        Directory.CreateDirectory(temporary);
        start.Environment["TEMP"] = temporary;
        start.Environment["TMP"] = temporary;
        process = Process.Start(start)!;
    }

    internal static string Text(nint window)
    {
        var buffer = new StringBuilder(32768);
        GetWindowText(window, buffer, buffer.Capacity);
        return buffer.ToString();
    }

    private List<nint> Windows()
    {
        var windows = new List<nint>();
        EnumWindows((window, _) =>
        {
            GetWindowThreadProcessId(window, out var id);
            if (id == process.Id) { SetWindowPos(window, 1, -32000, -32000, 0, 0, 0x0011); windows.Add(window); }
            return true;
        }, 0);
        return windows;
    }

    internal nint Window(string title) => Wait(() => Windows().FirstOrDefault(window => Text(window) == title), title);
    internal static List<nint> Children(nint parent)
    {
        var children = new List<nint>();
        EnumChildWindows(parent, (window, _) => { children.Add(window); return true; }, 0);
        return children;
    }

    internal nint Control(nint parent, string text) => Wait(() => Children(parent).FirstOrDefault(window => Text(window) == text), text);
    private static nint[] FindInputs(nint parent) => Children(parent).Where(window =>
    {
        var name = new StringBuilder(256);
        GetClassName(window, name, name.Capacity);
        return name.ToString().StartsWith("WindowsForms10.Edit.", StringComparison.OrdinalIgnoreCase);
    }).ToArray();

    internal nint[] Inputs(nint parent, int count)
    {
        try { Wait(() => FindInputs(parent).Length == count ? parent : 0, $"{count} input controls"); }
        catch
        {
            foreach (var child in Children(parent))
            {
                var name = new StringBuilder(256);
                GetClassName(child, name, name.Capacity);
                TestContext.Out.WriteLine($"{name}: {Text(child)}");
            }
            throw;
        }
        return FindInputs(parent);
    }

    internal static void Set(nint input, string value) => SendMessage(input, 0x000c, 0, value);
    internal static void Click(nint button) => Assert.That(PostMessage(button, 0x00f5, 0, 0), Is.True);
    internal static bool Enabled(nint control) => IsWindowEnabled(control);

    internal nint Open(string operation)
    {
        Click(Control(Window("Weave - Select Mode"), "Projects"));
        Click(Control(Window("Weave - Projects"), operation));
        return Window("Weave - MayaFlux Workspace Manager");
    }

    internal string Dismiss(string title)
    {
        var dialog = Window(title);
        var message = string.Join("\n", Children(dialog).Select(Text));
        Click(Control(dialog, "OK"));
        Wait(() => Windows().Any(window => Text(window) == title) ? 0 : 1, "closing " + title);
        return message;
    }

    internal void WaitForExit()
    {
        Assert.That(process.WaitForExit(10000), Is.True, "Packaged GUI did not exit after completing the operation.");
        Assert.That(process.ExitCode, Is.Zero);
    }

    private nint Wait(Func<nint> locate, string description)
    {
        var watch = Stopwatch.StartNew();
        while (watch.Elapsed < TimeSpan.FromSeconds(15))
        {
            var window = locate();
            if (window != 0) return window;
            if (process.HasExited) Assert.Fail($"Packaged GUI exited with {process.ExitCode} before {description}.");
            Thread.Sleep(50);
        }
        Assert.Fail($"Timed out waiting for {description}. Windows: {string.Join(", ", Windows().Select(Text))}");
        return 0;
    }

    public void Dispose()
    {
        foreach (var window in Windows()) PostMessage(window, 0x0010, 0, 0);
        if (!process.WaitForExit(3000)) { process.Kill(entireProcessTree: true); process.WaitForExit(3000); }
        process.Dispose();
    }
}
