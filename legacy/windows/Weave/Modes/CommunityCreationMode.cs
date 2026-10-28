using System;
using System.IO;
using System.Windows.Forms;
using Weave.Shared;
using Weave.UI.Project;

namespace Weave.Modes;

public class CommunityCreationMode : IWeaveMode
{
    public void Initialize(Panel container)
    {
        container.Controls.Clear();

        try
        {
            var mayaFluxRoot = Environment.GetEnvironmentVariable(WeaveConstants.ENV_MAYAFLUX_ROOT) ?? "C:\\MayaFlux";
            var templatesDir = Path.Combine(mayaFluxRoot, "share", "weave", "templates");

            if (!Directory.Exists(templatesDir))
            {
                ShowError(container, $"Templates directory not found at: {templatesDir}");
                return;
            }

            var view = new CommunityCreatorView(templatesDir);
            view.Dock = DockStyle.Fill;
            container.Controls.Add(view);
        }
        catch (Exception ex)
        {
            ShowError(container, $"Failed to load community creator: {ex.Message}");
        }
    }

    private void ShowError(Panel container, string message)
    {
        var errorLabel = new Label
        {
            Text = message,
            Font = new Font("Segoe UI", 11),
            Dock = DockStyle.Fill,
            TextAlign = ContentAlignment.MiddleCenter,
            ForeColor = Color.Red,
            Padding = new Padding(20)
        };
        container.Controls.Add(errorLabel);
    }
}