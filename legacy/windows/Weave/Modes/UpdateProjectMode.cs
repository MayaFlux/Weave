using System;
using System.Drawing;
using System.Windows.Forms;
using Weave.Theme;
using Weave.UI.Project;

namespace Weave.Modes;

public class UpdateProjectMode : IWeaveMode
{
    public void Initialize(Panel container)
    {
        container.Controls.Clear();

        try
        {
            var view = new AddCommunityView();
            view.Dock = DockStyle.Fill;
            container.Controls.Add(view);
        }
        catch (Exception ex)
        {
            var errorLabel = new Label
            {
                Text = $"Failed to load: {ex.Message}",
                Font = new Font("Segoe UI", 11),
                Dock = DockStyle.Fill,
                TextAlign = ContentAlignment.MiddleCenter,
                ForeColor = Color.Red,
                Padding = new Padding(20)
            };
            container.Controls.Add(errorLabel);
        }
    }
}