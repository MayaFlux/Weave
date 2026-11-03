//! Installed display-driver version advice. No graphics API or device probing.
use crate::operations::install::progress::Advisory;
use std::{sync::atomic::AtomicBool, time::Duration};

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Notice {
    vendor: String,
    message: String,
    url: String,
}

pub(crate) fn check(cancellation: &AtomicBool) -> Vec<Advisory> {
    let script = format!(
        "{}\nConvertTo-Json -InputObject @(Get-WeaveGraphicsDriverAdvisories) -Compress",
        crate::resources::WINDOWS_DRIVER_ADVISORY
    );
    check_with_command(&mut super::process::powershell(&script), cancellation)
}

fn check_with_command(
    command: &mut std::process::Command,
    cancellation: &AtomicBool,
) -> Vec<Advisory> {
    let result = crate::process::run_cancellable_with(
        &super::process::Runtime,
        command,
        false,
        Duration::from_secs(20),
        cancellation,
        &|_| {},
    );
    let parse = || -> Result<Vec<Advisory>, String> {
        let output = result?;
        if !output.status.success() {
            return Err("The installed-driver inventory was unavailable.".into());
        }
        let notices: Vec<Notice> = serde_json::from_slice(&output.stdout)
            .map_err(|_| "The installed-driver inventory returned unreadable data.".to_owned())?;
        Ok(notices
            .into_iter()
            .map(|notice| Advisory {
                vendor: notice.vendor,
                message: notice.message,
                url: notice.url,
            })
            .collect())
    };
    parse().unwrap_or_else(|_| vec![Advisory {
        vendor: "Graphics drivers".into(),
        message: "Installed graphics driver versions could not be checked. Check your graphics driver provider for updates, then retry. This does not prevent installation or project operations.".into(),
        url: String::new(),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_adapter_runs_shared_rules_with_controlled_driver_records() {
        let script = format!(
            r#"{}
function Get-CimInstance {{ @{{DriverProviderName='NVIDIA'; DriverVersion='32.0.15.7215'}} }}
ConvertTo-Json -InputObject @(Get-WeaveGraphicsDriverAdvisories) -Compress
"#,
            crate::resources::WINDOWS_DRIVER_ADVISORY
        );
        let notices = check_with_command(
            &mut super::super::process::powershell(&script),
            &AtomicBool::new(false),
        );
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].vendor, "NVIDIA");
        assert!(notices[0].message.contains("32.0.15.7215"));
        assert_eq!(notices[0].url, "https://www.nvidia.com/en-us/drivers/");
    }

    #[test]
    fn inventory_launch_failure_bad_output_and_cancellation_are_advisory() {
        for script in [
            "throw 'controlled inventory failure'",
            "Write-Output 'not JSON'",
        ] {
            let notices = check_with_command(
                &mut super::super::process::powershell(script),
                &AtomicBool::new(false),
            );
            assert_eq!(notices.len(), 1);
            assert!(notices[0].message.contains("could not be checked"));
        }
        let notices = check_with_command(
            &mut std::process::Command::new("weave-intentionally-missing-inventory.exe"),
            &AtomicBool::new(false),
        );
        assert!(notices[0].message.contains("could not be checked"));
        let notices = check_with_command(
            &mut super::super::process::powershell("Start-Sleep -Seconds 30"),
            &AtomicBool::new(true),
        );
        assert!(notices[0].message.contains("could not be checked"));
    }
}
