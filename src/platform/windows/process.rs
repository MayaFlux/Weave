//! Windows process preparation, PowerShell and descendant supervision.
use std::{ffi::c_void, os::windows::io::AsRawHandle, process::Child};

#[repr(C)]
#[derive(Default)]
struct BasicLimits {
    process_time: i64,
    job_time: i64,
    flags: u32,
    minimum_working_set: usize,
    maximum_working_set: usize,
    active_processes: u32,
    affinity: usize,
    priority: u32,
    scheduling: u32,
}
#[repr(C)]
#[derive(Default)]
struct ExtendedLimits {
    basic: BasicLimits,
    io: [u64; 6],
    process_memory: usize,
    job_memory: usize,
    peak_process_memory: usize,
    peak_job_memory: usize,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> *mut c_void;
    fn SetInformationJobObject(
        job: *mut c_void,
        class: i32,
        information: *const c_void,
        size: u32,
    ) -> i32;
    fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
    fn TerminateJobObject(job: *mut c_void, code: u32) -> i32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

pub struct ProbeJob(*mut c_void);
impl ProbeJob {
    pub fn attach(child: &mut Child) -> Result<Self, String> {
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            let _ = child.kill();
            return Err(format!(
                "Cannot supervise host-tool process: {}",
                std::io::Error::last_os_error()
            ));
        }
        let job = Self(handle);
        let mut limits = ExtendedLimits::default();
        limits.basic.flags = 0x2000;
        if unsafe {
            SetInformationJobObject(
                handle,
                9,
                &limits as *const _ as *const c_void,
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
            || unsafe { AssignProcessToJobObject(handle, child.as_raw_handle()) } == 0
        {
            let error = std::io::Error::last_os_error();
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Cannot supervise host-tool descendants: {error}"));
        }
        Ok(job)
    }
    pub fn terminate(&self) {
        unsafe {
            TerminateJobObject(self.0, 1);
        }
    }
}
impl Drop for ProbeJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

use std::os::windows::process::CommandExt;
use std::{path::PathBuf, process::Command};
pub fn powershell(script: &str) -> Command {
    let mut command =
        Command::new(system_directory().join("WindowsPowerShell/v1.0/powershell.exe"));
    command.args(["-NoProfile", "-NonInteractive", "-Command", &format!("$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); {script}")]);
    command
}

pub fn system_directory() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32")
}

pub struct Runtime;
impl crate::process::ProcessGuard for ProbeJob {
    fn terminate(&self) {
        ProbeJob::terminate(self);
    }
}
impl crate::process::ProcessRuntime for Runtime {
    type Guard = ProbeJob;
    fn prepare(&self, command: &mut Command) {
        command.creation_flags(0x08000000);
    }
    fn supervise(&self, child: &mut Child, installer: bool) -> Result<Option<ProbeJob>, String> {
        if installer {
            Ok(None)
        } else {
            ProbeJob::attach(child).map(Some)
        }
    }
    fn successful(&self, output: &std::process::Output, installer: bool) -> bool {
        output.status.success() || installer && output.status.code() == Some(3010)
    }
}
pub(crate) use crate::process::{checked, run};
