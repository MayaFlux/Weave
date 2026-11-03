fn main() {
    // Compose the same advisory into installation and standalone project scripts.
    // Generated projects need no Weave executable or sidecar files to validate.
    let scripts = std::path::Path::new("resources/templates/scripts/windows");
    for name in [
        "graphics-driver-rules.json",
        "graphics-driver-advisory.ps1",
        "validate.ps1",
    ] {
        println!("cargo:rerun-if-changed={}", scripts.join(name).display());
    }
    let rules = std::fs::read_to_string(scripts.join("graphics-driver-rules.json")).unwrap();
    let helper = std::fs::read_to_string(scripts.join("graphics-driver-advisory.ps1"))
        .unwrap()
        .replace("# @WEAVE_DRIVER_RULES@", &rules);
    let validation = std::fs::read_to_string(scripts.join("validate.ps1"))
        .unwrap()
        .replace("# @WEAVE_DRIVER_ADVISORY@", &helper);
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("graphics-driver-advisory.ps1"), helper).unwrap();
    std::fs::write(output.join("validate.ps1"), validation).unwrap();
    slint_build::compile("ui/main.slint").expect("failed to compile Weave UI");
}
