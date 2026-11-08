//! Inspects and updates the Lila CMake integration.

use super::{FeatureStatus, FileChange, OptionPlan, ProjectOptionError};
use crate::operations::project::{lila_debugger_path, lila_dll_copy_block, lila_link_block};
use std::fs;
use std::path::{Path, PathBuf};

/// Classifies the existing CMake integration without editing it.
pub(super) fn inspect_lila(path: &Path) -> Result<FeatureStatus, ProjectOptionError> {
    let cmake = fs::read_to_string(path).map_err(|error| {
        ProjectOptionError::Failure(format!("Could not read {}: {error}", path.display()))
    })?;
    let normalized = cmake.replace("\r\n", "\n");
    let has_link = normalized.contains(lila_link_block());
    let has_debugger_path = cmake.contains(lila_debugger_path());
    let has_host_runtime = cmake.contains("MayaFluxHost.dll")
        && cmake.contains("add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD");
    if has_link && has_debugger_path && has_host_runtime {
        return Ok(FeatureStatus::Enabled);
    }
    if !has_link
        && !has_debugger_path
        && !has_host_runtime
        && cmake.contains("# Optional: Lila (Live Coding JIT)")
        && cmake.contains("VS_DEBUGGER_ENVIRONMENT")
        && cmake.contains("# Windows subsystem configuration")
    {
        return Ok(FeatureStatus::Available);
    }
    Ok(FeatureStatus::Unknown(
        "Some Lila integration is present or the expected CMake sections were changed. Review cmake/mayaflux.cmake before editing.".to_owned(),
    ))
}

/// Finds one exact insertion anchor while retaining its line ending.
fn line_anchor(text: &str, marker: &str) -> Option<String> {
    let mut found = None;
    for newline in ["\r\n", "\n"] {
        let anchor = format!("{marker}{newline}");
        match text.matches(&anchor).count() {
            0 => {}
            1 if found.is_none() => found = Some(anchor),
            _ => return None,
        }
    }
    found
}

/// Inserts Lila blocks only when the expected CMake anchors are unambiguous.
pub(super) fn enable_lila(cmake: &str) -> Result<String, String> {
    let normalized = cmake.replace("\r\n", "\n");
    let has_link = normalized.contains(lila_link_block());
    let has_debugger = cmake.contains(lila_debugger_path());
    let has_host_runtime = cmake.contains("MayaFluxHost.dll")
        && cmake.contains("add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD");
    if has_link && has_debugger && has_host_runtime {
        return Ok(cmake.to_owned());
    }
    if has_link || has_debugger || has_host_runtime {
        return Err("Lila is partially configured in cmake/mayaflux.cmake. Review the existing integration and resolve it manually before retrying.".to_owned());
    }
    let link_anchor = line_anchor(cmake, "# Optional: Lila (Live Coding JIT)")
        .ok_or("Unsupported CMake layout: the Lila insertion section in cmake/mayaflux.cmake could not be identified safely.")?;
    let debugger_anchor = "$<TARGET_FILE_DIR:MayaFlux::MayaFluxLib>;";
    if cmake.matches(debugger_anchor).count() != 1 {
        return Err("Unsupported CMake layout: the Windows debugger path in cmake/mayaflux.cmake could not be identified safely.".to_owned());
    }
    let windows_anchor = line_anchor(cmake, "    # Windows subsystem configuration")
        .ok_or("Unsupported CMake layout: the Windows runtime section in cmake/mayaflux.cmake could not be identified safely.")?;

    let mut updated = cmake.replacen(
        &link_anchor,
        &format!("{link_anchor}{}", lila_link_block()),
        1,
    );
    updated = updated.replacen(
        debugger_anchor,
        &format!("{debugger_anchor}{}", lila_debugger_path()),
        1,
    );
    let dll_block = lila_dll_copy_block();
    let runtime_label = "    # WEAVE: Lila Windows runtime integration";
    match updated.matches(runtime_label).count() {
        1 => {
            let runtime_marker = line_anchor(&updated, runtime_label)
                .ok_or("The Lila runtime marker has no supported line ending.")?;
            let newline = if runtime_marker.ends_with("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            let empty_slot = format!("{runtime_marker}    {newline}");
            if updated.matches(&empty_slot).count() != 1 {
                return Err("The Lila Windows runtime section contains custom content. Review cmake/mayaflux.cmake before editing.".to_owned());
            }
            let replacement = format!("{runtime_marker}    {dll_block}{newline}");
            updated = updated.replacen(&empty_slot, &replacement, 1);
        }
        0 => {
            let insert_at = updated.find(&windows_anchor).ok_or_else(|| {
                "Unsupported CMake layout: the Windows runtime section changed during update."
                    .to_owned()
            })?;
            updated.insert_str(insert_at, &format!("    {dll_block}\n\n"));
        }
        _ => {
            return Err("Unsupported CMake layout: duplicate Lila runtime markers were found. Review cmake/mayaflux.cmake before editing.".to_owned());
        }
    }
    Ok(updated)
}

/// Plans a Lila edit without changing the project.
pub(super) fn plan(project: &Path) -> Result<OptionPlan, ProjectOptionError> {
    let mut plan = OptionPlan::default();
    let cmake_path = project.join("cmake/mayaflux.cmake");
    let cmake = fs::read_to_string(&cmake_path).map_err(|error| {
        ProjectOptionError::Failure(format!("Could not read {}: {error}", cmake_path.display()))
    })?;
    let updated = enable_lila(&cmake).map_err(ProjectOptionError::Failure)?;
    if updated != cmake {
        plan.changes.push(FileChange {
            relative: PathBuf::from("cmake/mayaflux.cmake"),
            contents: updated.into_bytes(),
        });
        plan.messages
            .push("Enabled Lila integration in cmake/mayaflux.cmake.".to_owned());
    } else {
        plan.messages
            .push("Lila integration is already enabled.".to_owned());
    }
    Ok(plan)
}
