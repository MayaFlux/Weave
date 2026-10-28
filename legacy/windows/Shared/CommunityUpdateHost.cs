using System.Diagnostics;

namespace Weave.Shared;

public class CommunityUpdateHost
{
    public virtual TimeSpan CloneTimeout => TimeSpan.FromMinutes(5);
    public virtual Process StartProcess(ProcessStartInfo request) => Process.Start(request)
        ?? throw new InvalidOperationException("Could not start Git. Install Git for Windows and retry.");
    public virtual FileAttributes GetAttributes(string path) => File.GetAttributes(path);
    public virtual void MoveRegistration(string source, string destination) => File.Move(source, destination, true);
    public virtual void DeleteIncompleteClone(string path) => Directory.Delete(path, true);
}
