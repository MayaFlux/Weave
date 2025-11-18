using System.Collections.Concurrent;

namespace Weave.Utils;

public class Logger
{
    private readonly ConcurrentQueue<string> logQueue = new();
    private readonly string logFilePath;
    private readonly object lockObj = new();

    public Logger(string? customPath = null)
    {
        logFilePath = customPath ?? Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "weave_install.log"
        );
        
        InitializeLogFile();
    }

    private void InitializeLogFile()
    {
        try
        {
            File.WriteAllText(logFilePath, $"=== Weave Installation Log ===\nStarted: {DateTime.Now:yyyy-MM-dd HH:mm:ss}\n\n");
        }
        catch { /* Ignore if can't write */ }
    }

    public void Log(string message)
    {
        var timestamp = DateTime.Now.ToString("HH:mm:ss");
        var formattedMessage = $"[{timestamp}] {message}";
        
        logQueue.Enqueue(formattedMessage);
        
        try
        {
            lock (lockObj)
            {
                File.AppendAllText(logFilePath, formattedMessage + Environment.NewLine);
            }
        }
        catch { /* Ignore if can't write */ }
    }

    public string GetLogFile() => logFilePath;
    
    public IEnumerable<string> GetLogs() => logQueue.ToList();
}
