#![cfg(target_os = "windows")]

use std::{fs, path::PathBuf, process::Command};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "weave driver advisory {}",
            weave::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let executable = std::env::var_os("WEAVE_TEST_EXECUTABLE")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_weave").into());
        let result = Command::new(executable)
            .args(["new", "DriverAdvisory"])
            .arg(&root)
            .arg("--no-vscode")
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Self(root)
    }
    fn run(&self, body: &str) -> std::process::Output {
        fs::write(self.0.join("check.ps1"), format!("$ErrorActionPreference='Stop'\n. (Join-Path $PSScriptRoot 'DriverAdvisory/validate.ps1')\n{body}")).unwrap();
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(self.0.join("check.ps1"))
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn bundled_advisory_checks_all_vendors_and_numeric_version_boundaries_without_querying_devices() {
    let fixture = Fixture::new();
    let result = fixture.run(
        r#"
function Get-CimInstance { throw 'A supplied inventory must not query the host' }
$drivers = @(
    @{ DriverProviderName='NVIDIA'; DriverVersion='32.0.15.7215'; InfName='old.inf' },
    @{ DriverProviderName='NVIDIA'; DriverVersion='32.0.15.7215'; InfName='duplicate.inf' },
    @{ DriverProviderName='NVIDIA'; DriverVersion='32.0.15.7216' },
    @{ DriverProviderName='NVIDIA'; DriverVersion='32.0.15.9000' },
    @{ DriverProviderName='Advanced Micro Devices, Inc.'; DriverVersion='32.0.22029.1018' },
    @{ DriverProviderName='Advanced Micro Devices, Inc.'; DriverVersion='32.0.22029.1019' },
    @{ DriverProviderName='Advanced Micro Devices, Inc.'; DriverVersion='32.0.21045.20006' },
    @{ DriverProviderName='Intel Corporation'; DriverVersion='32.0.101.9000' },
    @{ DriverProviderName='NVIDIA'; DriverVersion='invalid' },
    @{ DriverProviderName='Microsoft'; DriverVersion='1.0.0.0' }
)
ConvertTo-Json -InputObject @(Get-WeaveGraphicsDriverAdvisories -Drivers $drivers) -Compress
"#,
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let notices: Vec<serde_json::Value> = serde_json::from_slice(&result.stdout).unwrap();
    let old: Vec<_> = notices.iter().filter(|n| n["Kind"] == "outdated").collect();
    assert_eq!(old.len(), 3, "{notices:?}");
    assert!(old.iter().any(
        |n| n["Vendor"] == "NVIDIA" && n["Message"].as_str().unwrap().contains("32.0.15.7215")
    ));
    assert!(old.iter().any(
        |n| n["Vendor"] == "AMD" && n["Message"].as_str().unwrap().contains("32.0.21045.20006")
    ));
    assert!(
        notices
            .iter()
            .any(|n| n["Vendor"] == "Intel" && n["Kind"] == "unknown")
    );
    assert!(
        notices
            .iter()
            .any(|n| n["Vendor"] == "NVIDIA" && n["Kind"] == "unknown")
    );
    assert!(
        notices
            .iter()
            .all(|n| n["Url"].as_str().unwrap().starts_with("https://"))
    );
}

#[test]
fn bundled_advisory_handles_absence_and_inventory_failure_without_failing_validation() {
    let fixture = Fixture::new();
    let result = fixture.run(r#"
function Get-CimInstance { throw 'controlled inventory failure' }
if (@(Get-WeaveGraphicsDriverAdvisories -Drivers @()).Count -ne 0) { throw 'Absent drivers produced a notice' }
$script:validationIssues = 0
$script:validationWarnings = 0
Write-GraphicsDriverAdvisory
if ($script:validationIssues -ne 0) { throw 'Driver advice became a validation failure' }
if ($script:validationWarnings -ne 1) { throw 'Inventory failure was not reported' }
"#);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("could not be checked"));
}

#[test]
fn standalone_validation_exit_status_is_unchanged_by_old_drivers_or_inventory_failure() {
    let fixture = Fixture::new();
    for inventory in [
        "@{DriverProviderName='NVIDIA'; DriverVersion='32.0.15.7215'}",
        "throw 'controlled inventory failure'",
    ] {
        let result = fixture.run(&format!(
            r#"
function Get-CimInstance {{ {inventory} }}
function Get-ValidationEnvironment {{ return $null }}
function Invoke-ValidationSection {{ }}
$code = Invoke-InstallValidation -Root 'C:\Controlled MayaFlux'
if ($code -ne 0) {{ throw "Graphics advice changed exit status to $code" }}
if ($script:validationWarnings -ne 1) {{ throw 'Expected one advisory warning' }}
"#
        ));
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
