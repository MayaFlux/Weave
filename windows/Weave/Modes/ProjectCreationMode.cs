namespace Weave.Modes;

public class ProjectCreationMode : IWeaveMode
{
    public void Initialize(Panel container)
    {
        container.Controls.Clear();

        var label = new Label
        {
            Text = "Project Creation Mode (Not Yet Implemented)",
            Font = new Font("Segoe UI", 12),
            Dock = DockStyle.Fill,
            TextAlign = ContentAlignment.MiddleCenter,
            ForeColor = Color.Gray
        };

        container.Controls.Add(label);
    }
}