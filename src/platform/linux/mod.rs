use crate::operations::install::InstallError;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Distribution {
    Arch,
    Fedora,
    Ubuntu,
}

impl Distribution {
    fn detect_with(available: impl Fn(&str) -> bool) -> Result<Self, String> {
        if available("pacman") {
            Ok(Self::Arch)
        } else if available("dnf") {
            Ok(Self::Fedora)
        } else if available("apt-get") {
            Ok(Self::Ubuntu)
        } else {
            Err(
                "Unsupported Linux distribution: pacman, dnf, and apt-get were not found."
                    .to_owned(),
            )
        }
    }
}

struct InstallPlan {
    distribution: Distribution,
    package: &'static str,
    conflicts: &'static [&'static str],
    development: bool,
}

impl InstallPlan {
    fn new(distribution: Distribution, channel: &str) -> Result<Self, String> {
        let development = match channel {
            "stable" => false,
            "development" => true,
            _ => return Err(format!("Unsupported MayaFlux install channel: {channel}")),
        };
        let (package, conflicts): (_, &'static [&'static str]) = match (distribution, development) {
            (Distribution::Arch, false) => ("mayaflux-bin", &["mayaflux", "mayaflux-dev-bin"]),
            (Distribution::Arch, true) => ("mayaflux-dev-bin", &["mayaflux", "mayaflux-bin"]),
            (Distribution::Fedora, false) => ("mayaflux", &["mayaflux-dev"]),
            (Distribution::Fedora, true) => ("mayaflux-dev", &["mayaflux"]),
            (Distribution::Ubuntu, false) => ("mayaflux", &["mayaflux-edge"]),
            (Distribution::Ubuntu, true) => ("mayaflux-edge", &["mayaflux"]),
        };
        Ok(Self {
            distribution,
            package,
            conflicts,
            development,
        })
    }

    fn install_commands(&self, checkout: Option<&Path>) -> Vec<Vec<String>> {
        let commands: Vec<Vec<&str>> = match self.distribution {
            Distribution::Arch => {
                let checkout = checkout.expect("Arch requires a temporary checkout");
                return vec![
                    vec![
                        "git".into(),
                        "clone".into(),
                        format!("https://aur.archlinux.org/{}.git", self.package),
                        checkout.display().to_string(),
                    ],
                    vec![
                        "makepkg".into(),
                        "--dir".into(),
                        checkout.display().to_string(),
                        "-si".into(),
                        "--noconfirm".into(),
                    ],
                ];
            }
            Distribution::Fedora => vec![
                vec![
                    "sudo",
                    "dnf",
                    "copr",
                    "enable",
                    "-y",
                    "ranjithshegde/spirv-cross",
                ],
                vec![
                    "sudo",
                    "dnf",
                    "copr",
                    "enable",
                    "-y",
                    "ranjithshegde/asio-standalone",
                ],
                vec![
                    "sudo",
                    "dnf",
                    "copr",
                    "enable",
                    "-y",
                    if self.development {
                        "ranjithshegde/mayaflux-dev"
                    } else {
                        "ranjithshegde/mayaflux"
                    },
                ],
                vec!["sudo", "dnf", "install", "-y", self.package],
            ],
            Distribution::Ubuntu => vec![
                vec![
                    "sudo",
                    "add-apt-repository",
                    "-y",
                    if self.development {
                        "ppa:mayaflux/mayaflux-dev"
                    } else {
                        "ppa:mayaflux/mayaflux"
                    },
                ],
                vec!["sudo", "apt-get", "update"],
                vec!["sudo", "apt-get", "install", "-y", self.package],
            ],
        };
        commands
            .into_iter()
            .map(|cmd| cmd.into_iter().map(str::to_owned).collect())
            .collect()
    }

    fn remove_command(&self, package: &str) -> Vec<String> {
        let command = match self.distribution {
            Distribution::Arch => vec!["sudo", "pacman", "-R", "--noconfirm", package],
            Distribution::Fedora => vec!["sudo", "dnf", "remove", "-y", package],
            Distribution::Ubuntu => vec!["sudo", "apt-get", "remove", "-y", package],
        };
        command.into_iter().map(str::to_owned).collect()
    }
}

trait InstallHost {
    fn available(&self, name: &str) -> bool;
    fn installed(&self, distro: Distribution, package: &str) -> Result<bool, String>;
    fn execute(
        &self,
        command: &[String],
        password: Option<&str>,
        log: &(impl Fn(&str) + Sync),
    ) -> Result<(), String>;
}

struct SystemHost;

impl InstallHost for SystemHost {
    fn available(&self, name: &str) -> bool {
        executable_on_path(name)
    }

    fn installed(&self, distro: Distribution, package: &str) -> Result<bool, String> {
        let mut command = match distro {
            Distribution::Arch => {
                let mut c = Command::new("pacman");
                c.args(["-Qq", package]);
                c
            }
            Distribution::Fedora => {
                let mut c = Command::new("rpm");
                c.args(["-q", package]);
                c
            }
            Distribution::Ubuntu => {
                let mut c = Command::new("dpkg-query");
                c.args(["-W", "-f=${Status}", package]);
                c
            }
        };
        let output = command
            .env_remove("LD_LIBRARY_PATH")
            .env_remove("LD_PRELOAD")
            .output()
            .map_err(|error| format!("Could not check package {package}: {error}"))?;
        if !output.status.success() {
            return Ok(false);
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(query_output_is_installed(distro, package, &stdout))
    }

    fn execute(
        &self,
        command: &[String],
        password: Option<&str>,
        log: &(impl Fn(&str) + Sync),
    ) -> Result<(), String> {
        run_linux_command(command, password, log)
    }
}

fn query_output_is_installed(distro: Distribution, package: &str, stdout: &str) -> bool {
    match distro {
        Distribution::Arch => stdout.lines().any(|line| line == package),
        Distribution::Fedora => true,
        Distribution::Ubuntu => stdout.contains("install ok installed"),
    }
}

fn executable_on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| is_executable(&directory.join(name)))
    })
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

struct TempCheckout(PathBuf);

impl TempCheckout {
    fn create() -> Result<Self, String> {
        let root = std::env::temp_dir();
        for attempt in 0..100 {
            let path = root.join(format!(
                "weave-aur-{}-{}-{attempt}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|error| error.to_string())?
                    .as_nanos()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!("Could not make a temporary AUR checkout: {error}"));
                }
            }
        }
        Err("Could not make a unique temporary AUR checkout.".to_owned())
    }
}

impl Drop for TempCheckout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn install_linux(
    channel: &str,
    replace_conflict: bool,
    password: Option<&str>,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, InstallError> {
    install_linux_with_host(channel, replace_conflict, password, log, &SystemHost)
}

pub fn check_linux_conflict(channel: &str) -> Result<Option<String>, InstallError> {
    check_linux_conflict_with_host(channel, &SystemHost)
}

fn check_linux_conflict_with_host(
    channel: &str,
    host: &impl InstallHost,
) -> Result<Option<String>, InstallError> {
    let (_, conflicts) = prepare_linux_install(channel, host)?;
    Ok((!conflicts.is_empty()).then(|| conflicts.join(", ")))
}

fn install_linux_with_host(
    channel: &str,
    replace_conflict: bool,
    password: Option<&str>,
    log: &(impl Fn(&str) + Sync),
    host: &impl InstallHost,
) -> Result<String, InstallError> {
    let (plan, installed_conflicts) = prepare_linux_install(channel, host)?;
    let distro = plan.distribution;
    log(&format!(
        "Detected distribution: {distro:?}. Selected package: {}",
        plan.package
    ));
    if !installed_conflicts.is_empty() && !replace_conflict {
        return Err(InstallError::Conflict {
            installed_formula: installed_conflicts.join(", "),
        });
    }
    let checkout = if distro == Distribution::Arch {
        Some(TempCheckout::create().map_err(InstallError::Failed)?)
    } else {
        None
    };
    let mut removed = Vec::new();
    for conflict in installed_conflicts {
        let command = plan.remove_command(conflict);
        log(&format!("Removing conflicting package {conflict}..."));
        host.execute(&command, password, log).map_err(|error| {
            let recovery = if removed.is_empty() { String::new() } else {
                format!(" Package(s) {} were already removed; restore them with your package manager or retry.", removed.join(", "))
            };
            InstallError::Failed(format!("Could not remove {conflict}: {error}.{recovery}"))
        })?;
        removed.push(conflict);
    }
    for command in plan.install_commands(checkout.as_ref().map(|value| value.0.as_path())) {
        log(&format!("Running: {}", command.join(" ")));
        let credential = if command[0] == "sudo" || command[0] == "makepkg" {
            password
        } else {
            None
        };
        if let Err(error) = host.execute(&command, credential, log) {
            let recovery = if removed.is_empty() {
                String::new()
            } else {
                format!(
                    " Previous package(s) {} were removed; restore them with your package manager or retry this install.",
                    removed.join(", ")
                )
            };
            return Err(InstallError::Failed(format!(
                "Command failed: {}: {error}.{recovery}",
                command.join(" ")
            )));
        }
    }
    let recovery = if removed.is_empty() {
        String::new()
    } else {
        format!(
            " Previous package(s) {} were removed; restore them with your package manager or retry this install.",
            removed.join(", ")
        )
    };
    let installed = host.installed(distro, plan.package).map_err(|error| {
        InstallError::Failed(format!(
            "Could not verify {}: {error}.{recovery}",
            plan.package
        ))
    })?;
    if !installed {
        return Err(InstallError::Failed(format!(
            "Package commands completed, but '{}' is not listed as installed. Check your package manager and retry.{recovery}",
            plan.package,
        )));
    }
    Ok(format!("{} installed and verified.", plan.package))
}

fn prepare_linux_install(
    channel: &str,
    host: &impl InstallHost,
) -> Result<(InstallPlan, Vec<&'static str>), InstallError> {
    // Validate the channel before probing or changing the host.
    if channel != "stable" && channel != "development" {
        return Err(InstallError::Failed(format!(
            "Unsupported MayaFlux install channel: {channel}"
        )));
    }
    let distro =
        Distribution::detect_with(|name| host.available(name)).map_err(InstallError::Failed)?;
    let plan = InstallPlan::new(distro, channel).map_err(InstallError::Failed)?;
    let required_tools: &[&str] = match distro {
        Distribution::Arch => &["git", "makepkg", "sudo"],
        Distribution::Fedora => &["rpm", "sudo"],
        Distribution::Ubuntu => &["dpkg-query", "add-apt-repository", "sudo"],
    };
    for tool in required_tools {
        if !host.available(tool) {
            return Err(InstallError::Failed(format!(
                "Required host tool '{tool}' was not found. Install it and retry MayaFlux installation."
            )));
        }
    }
    let installed_conflicts: Vec<_> = plan
        .conflicts
        .iter()
        .filter_map(|package| match host.installed(distro, package) {
            Ok(true) => Some(Ok(*package)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<_, _>>()
        .map_err(InstallError::Failed)?;
    Ok((plan, installed_conflicts))
}

fn run_linux_command(
    command: &[String],
    password: Option<&str>,
    log: &(impl Fn(&str) + Sync),
) -> Result<(), String> {
    run_linux_command_with_limits(
        command,
        password,
        log,
        Duration::from_secs(300),
        Duration::from_secs(1800),
    )
}

fn transfer_amount(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("[#") {
        return rest.split_whitespace().nth(1)?.split('/').next();
    }
    let parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() >= 5 && parts[0].parse::<u8>().is_ok() && parts[2].parse::<u8>().is_ok() {
        return Some(parts[3]);
    }
    None
}

fn strip_ansi(bytes: &[u8]) -> String {
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == 0x1b && index + 1 < bytes.len() && bytes[index + 1] == b'[' {
            index += 2;
            while index < bytes.len() && !(0x40..=0x7e).contains(&bytes[index]) {
                index += 1;
            }
            index += 1;
        } else if bytes[index] == 0x1b && index + 1 < bytes.len() && bytes[index + 1] == b'(' {
            index += 3;
        } else if bytes[index] == 0x1b {
            index += 1;
        } else {
            result.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn run_piped_command_with_limits(
    mut process: Command,
    label: &str,
    log: &(impl Fn(&str) + Sync),
    download_timeout: Duration,
    command_timeout: Duration,
) -> Result<(), String> {
    use std::sync::mpsc::{self, RecvTimeoutError};

    process
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = process
        .spawn()
        .map_err(|error| format!("Could not run {label}: {error}"))?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let (sender, receiver) = mpsc::channel::<String>();
    std::thread::scope(|scope| {
        let stdout_sender = sender.clone();
        scope.spawn(move || {
            crate::operations::install::stream_output(stdout, &|line| {
                let _ = stdout_sender.send(line.to_owned());
            });
        });
        scope.spawn(move || {
            crate::operations::install::stream_output(stderr, &|line| {
                let _ = sender.send(line.to_owned());
            });
        });
        let mut completed = None;
        let mut last_output = Instant::now();
        let mut last_progress = Instant::now();
        let mut received_amount: Option<String> = None;
        let mut downloading = false;
        loop {
            match receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(line) => {
                    if let Some(amount) = transfer_amount(&line) {
                        downloading = true;
                        if received_amount.as_deref() != Some(amount) {
                            received_amount = Some(amount.to_owned());
                            last_progress = Instant::now();
                            log(&line);
                        }
                    } else if !line.trim().is_empty() {
                        if !downloading
                            || ["==>", "  ->", "::"]
                                .iter()
                                .any(|prefix| line.starts_with(prefix))
                        {
                            downloading = false;
                            received_amount = None;
                            last_output = Instant::now();
                        }
                        log(&line);
                    }
                }
                Err(RecvTimeoutError::Disconnected) if completed.is_some() => break,
                Err(RecvTimeoutError::Disconnected) => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
            if completed.is_none() {
                completed = child
                    .try_wait()
                    .map_err(|error| format!("Could not wait for {label}: {error}"))?;
            }
            if downloading && last_progress.elapsed() >= download_timeout
                || !downloading && last_output.elapsed() >= command_timeout
            {
                unsafe { libc::killpg(child.id() as i32, libc::SIGTERM) };
                std::thread::sleep(Duration::from_millis(200));
                if child.try_wait().ok().flatten().is_none() {
                    unsafe { libc::killpg(child.id() as i32, libc::SIGKILL) };
                }
                let _ = child.wait();
                return Err(if downloading {
                    format!(
                        "No download progress for 5 minutes while running {label}; retry the installation when the connection is available."
                    )
                } else {
                    format!(
                        "No output for 30 minutes while running {label}; the command was stopped."
                    )
                });
            }
        }
        let status = completed.expect("child exited before output closed");
        if status.success() {
            Ok(())
        } else {
            Err(format!("{label} exited with {status}"))
        }
    })
}

fn run_linux_command_with_limits(
    command: &[String],
    password: Option<&str>,
    log: &(impl Fn(&str) + Sync),
    download_timeout: Duration,
    command_timeout: Duration,
) -> Result<(), String> {
    let label = command.join(" ");
    let mut process = Command::new(&command[0]);
    process
        .args(&command[1..])
        .env_remove("LD_LIBRARY_PATH")
        .env_remove("LD_PRELOAD");
    if password.is_none() {
        return run_piped_command_with_limits(
            process,
            &label,
            log,
            download_timeout,
            command_timeout,
        );
    }
    process.env("SUDO_PROMPT", "WEAVE_AUTHORIZATION:");

    let mut master_fd = -1;
    let mut slave_fd = -1;
    if unsafe {
        libc::openpty(
            &mut master_fd,
            &mut slave_fd,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        )
    } != 0
    {
        return Err(format!(
            "Could not open installer terminal: {}",
            io::Error::last_os_error()
        ));
    }
    let mut master = unsafe { std::fs::File::from_raw_fd(master_fd) };
    let slave = unsafe { std::fs::File::from_raw_fd(slave_fd) };
    let mut terminal = unsafe { std::mem::zeroed::<libc::termios>() };
    if unsafe { libc::tcgetattr(slave.as_raw_fd(), &mut terminal) } != 0 {
        return Err(format!(
            "Could not configure installer terminal: {}",
            io::Error::last_os_error()
        ));
    }
    terminal.c_lflag &= !libc::ECHO;
    if unsafe { libc::tcsetattr(slave.as_raw_fd(), libc::TCSANOW, &terminal) } != 0 {
        return Err(format!(
            "Could not disable installer terminal echo: {}",
            io::Error::last_os_error()
        ));
    }
    let size = libc::winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe { libc::ioctl(slave.as_raw_fd(), libc::TIOCSWINSZ, &size) };
    process
        .stdin(slave.try_clone().map_err(|error| error.to_string())?)
        .stdout(slave.try_clone().map_err(|error| error.to_string())?)
        .stderr(slave.try_clone().map_err(|error| error.to_string())?);
    unsafe {
        process.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = process
        .spawn()
        .map_err(|error| format!("Could not run {label}: {error}"))?;
    drop(slave);
    let flags = unsafe { libc::fcntl(master.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "Could not read installer terminal: {}",
            io::Error::last_os_error()
        ));
    }
    let password = password.expect("password branch");
    let mut pending = Vec::new();
    let mut prompt_answered = false;
    let mut last_output = Instant::now();
    let mut last_progress = Instant::now();
    let mut received_amount: Option<String> = None;
    let mut downloading = false;
    let mut completed = None;
    let completed = loop {
        let mut bytes = [0_u8; 4096];
        let mut drained = false;
        match master.read(&mut bytes) {
            Ok(0) => drained = true,
            Ok(count) => {
                for byte in &bytes[..count] {
                    if *byte == b'\n' || *byte == b'\r' {
                        if !pending.is_empty() {
                            let line = strip_ansi(&pending).replace(password, "[redacted]");
                            let line = line.trim_end();
                            if let Some(amount) = transfer_amount(line) {
                                downloading = true;
                                if received_amount.as_deref() != Some(amount) {
                                    received_amount = Some(amount.to_owned());
                                    last_progress = Instant::now();
                                    log(line);
                                }
                            } else if !line.trim().is_empty() {
                                if !downloading
                                    || ["==>", "  ->", "::"]
                                        .iter()
                                        .any(|prefix| line.starts_with(prefix))
                                {
                                    downloading = false;
                                    received_amount = None;
                                    last_output = Instant::now();
                                }
                                log(line);
                            }
                            pending.clear();
                        }
                        prompt_answered = false;
                    } else {
                        pending.push(*byte);
                    }
                }
                let prompt = strip_ansi(&pending).to_ascii_lowercase();
                if !prompt_answered {
                    let prompt = prompt.trim_end();
                    if prompt.ends_with("[y/n]:") {
                        master.write_all(b"y\n").map_err(|error| {
                            format!("Could not confirm package manager prompt: {error}")
                        })?;
                        prompt_answered = true;
                        last_output = Instant::now();
                    } else if (prompt.contains("password")
                        || prompt.contains("weave_authorization"))
                        && prompt.ends_with(':')
                    {
                        master
                            .write_all(password.as_bytes())
                            .and_then(|_| master.write_all(b"\n"))
                            .map_err(|error| {
                                format!("Could not answer authorization prompt: {error}")
                            })?;
                        prompt_answered = true;
                        last_output = Instant::now();
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => drained = true,
            Err(error) if error.raw_os_error() == Some(libc::EIO) => drained = true,
            Err(error) => return Err(format!("Could not read installer output: {error}")),
        }
        if completed.is_none() {
            completed = child
                .try_wait()
                .map_err(|error| format!("Could not wait for {label}: {error}"))?;
        }
        if drained && let Some(status) = completed {
            if !pending.is_empty() {
                let line = strip_ansi(&pending).replace(password, "[redacted]");
                if !line.trim().is_empty() {
                    log(line.trim_end());
                }
            }
            break status;
        }
        if downloading && last_progress.elapsed() >= download_timeout
            || !downloading && last_output.elapsed() >= command_timeout
        {
            unsafe { libc::killpg(child.id() as i32, libc::SIGTERM) };
            std::thread::sleep(Duration::from_millis(200));
            if child.try_wait().ok().flatten().is_none() {
                unsafe { libc::killpg(child.id() as i32, libc::SIGKILL) };
            }
            let _ = child.wait();
            return Err(if downloading {
                format!(
                    "No download progress for 5 minutes while running {label}; retry the installation when the connection is available."
                )
            } else {
                format!(
                    "No installer output for 30 minutes while running {label}; retry the installation."
                )
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if completed.success() {
        Ok(())
    } else {
        Err(format!("{label} failed"))
    }
}

pub fn shader_source_root() -> Option<PathBuf> {
    ["/usr", "/usr/local"]
        .into_iter()
        .map(|prefix| Path::new(prefix).join("share/MayaFlux/src/shaders"))
        .find(|path| path.is_dir())
}

#[cfg(test)]
mod install_tests {
    use super::{
        Distribution, InstallHost, InstallPlan, check_linux_conflict_with_host,
        install_linux_with_host, is_executable, query_output_is_installed, run_linux_command,
        run_linux_command_with_limits,
    };
    use crate::operations::install::InstallError;
    use std::path::Path;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    struct FakeHost {
        distro: Distribution,
        installed: Mutex<Vec<String>>,
        commands: Mutex<Vec<Vec<String>>>,
        password_use: Mutex<Vec<bool>>,
        fail_on: Option<String>,
        query_error_on: Option<String>,
    }

    impl FakeHost {
        fn new(distro: Distribution, installed: &[&str]) -> Self {
            Self {
                distro,
                installed: Mutex::new(installed.iter().map(|s| (*s).to_owned()).collect()),
                commands: Mutex::new(Vec::new()),
                password_use: Mutex::new(Vec::new()),
                fail_on: None,
                query_error_on: None,
            }
        }
    }

    impl InstallHost for FakeHost {
        fn available(&self, name: &str) -> bool {
            if [
                "sudo",
                "git",
                "makepkg",
                "rpm",
                "dpkg-query",
                "add-apt-repository",
            ]
            .contains(&name)
            {
                return true;
            }
            name == match self.distro {
                Distribution::Arch => "pacman",
                Distribution::Fedora => "dnf",
                Distribution::Ubuntu => "apt-get",
            }
        }
        fn installed(&self, _distro: Distribution, package: &str) -> Result<bool, String> {
            if self.query_error_on.as_deref() == Some(package) {
                return Err(format!("Could not query {package}"));
            }
            Ok(self
                .installed
                .lock()
                .unwrap()
                .iter()
                .any(|item| item == package))
        }
        fn execute(
            &self,
            command: &[String],
            password: Option<&str>,
            _log: &(impl Fn(&str) + Sync),
        ) -> Result<(), String> {
            self.commands.lock().unwrap().push(command.to_vec());
            self.password_use.lock().unwrap().push(password.is_some());
            if self
                .fail_on
                .as_ref()
                .is_some_and(|fragment| command.join(" ").contains(fragment))
            {
                return Err("simulated command failure".to_owned());
            }
            if command.first().is_some_and(|arg| arg == "git") {
                let checkout = Path::new(command.last().unwrap());
                assert!(checkout.is_dir());
                std::fs::write(checkout.join("PKGBUILD"), "fake package").unwrap();
            }
            let mut installed = self.installed.lock().unwrap();
            if command.iter().any(|arg| arg == "remove" || arg == "-R") {
                installed.retain(|item| item != command.last().unwrap());
            } else if command.iter().any(|arg| arg == "install") {
                installed.push(command.last().unwrap().to_owned());
            }
            Ok(())
        }
    }

    #[test]
    fn detects_legacy_package_managers_in_priority_order() {
        assert_eq!(
            Distribution::detect_with(|_| true).unwrap(),
            Distribution::Arch
        );
        assert_eq!(
            Distribution::detect_with(|name| name != "pacman").unwrap(),
            Distribution::Fedora
        );
        assert_eq!(
            Distribution::detect_with(|name| name == "apt-get").unwrap(),
            Distribution::Ubuntu
        );
        assert!(Distribution::detect_with(|_| false).is_err());
    }

    #[test]
    fn arch_uses_fresh_aur_checkout_and_preserves_both_conflict_checks() {
        let plan = InstallPlan::new(Distribution::Arch, "development").unwrap();
        assert_eq!(plan.package, "mayaflux-dev-bin");
        assert_eq!(plan.conflicts, &["mayaflux", "mayaflux-bin"]);
        assert_eq!(
            plan.install_commands(Some(Path::new("/tmp/aur-checkout"))),
            vec![
                vec![
                    "git",
                    "clone",
                    "https://aur.archlinux.org/mayaflux-dev-bin.git",
                    "/tmp/aur-checkout"
                ],
                vec![
                    "makepkg",
                    "--dir",
                    "/tmp/aur-checkout",
                    "-si",
                    "--noconfirm"
                ],
            ]
        );
        assert_eq!(
            plan.remove_command("mayaflux"),
            vec!["sudo", "pacman", "-R", "--noconfirm", "mayaflux"]
        );
    }

    #[test]
    fn fedora_and_ubuntu_keep_legacy_repository_and_install_order() {
        let fedora = InstallPlan::new(Distribution::Fedora, "stable").unwrap();
        assert_eq!(fedora.conflicts, &["mayaflux-dev"]);
        assert_eq!(
            fedora.install_commands(None),
            vec![
                vec![
                    "sudo",
                    "dnf",
                    "copr",
                    "enable",
                    "-y",
                    "ranjithshegde/spirv-cross"
                ],
                vec![
                    "sudo",
                    "dnf",
                    "copr",
                    "enable",
                    "-y",
                    "ranjithshegde/asio-standalone"
                ],
                vec![
                    "sudo",
                    "dnf",
                    "copr",
                    "enable",
                    "-y",
                    "ranjithshegde/mayaflux"
                ],
                vec!["sudo", "dnf", "install", "-y", "mayaflux"],
            ]
        );
        let ubuntu = InstallPlan::new(Distribution::Ubuntu, "development").unwrap();
        assert_eq!(ubuntu.conflicts, &["mayaflux"]);
        assert_eq!(
            ubuntu.install_commands(None),
            vec![
                vec![
                    "sudo",
                    "add-apt-repository",
                    "-y",
                    "ppa:mayaflux/mayaflux-dev"
                ],
                vec!["sudo", "apt-get", "update"],
                vec!["sudo", "apt-get", "install", "-y", "mayaflux-edge"],
            ]
        );
        assert_eq!(
            ubuntu.remove_command("mayaflux"),
            vec!["sudo", "apt-get", "remove", "-y", "mayaflux"]
        );
    }

    #[test]
    fn invalid_channel_is_rejected_before_commands_are_constructed() {
        assert!(InstallPlan::new(Distribution::Ubuntu, "nightly").is_err());
        let host = FakeHost::new(Distribution::Ubuntu, &[]);
        let error = check_linux_conflict_with_host("nightly", &host).unwrap_err();
        assert!(error.to_string().contains("nightly"));
        assert!(host.commands.lock().unwrap().is_empty());
    }

    #[test]
    fn plans_cover_each_channel_without_marking_its_own_package_as_a_conflict() {
        for distro in [
            Distribution::Arch,
            Distribution::Fedora,
            Distribution::Ubuntu,
        ] {
            for channel in ["stable", "development"] {
                let plan = InstallPlan::new(distro, channel).unwrap();
                assert!(
                    !plan.conflicts.contains(&plan.package),
                    "{distro:?} {channel}"
                );
                let host = FakeHost::new(distro, &[plan.package]);
                assert_eq!(
                    check_linux_conflict_with_host(channel, &host).unwrap(),
                    None,
                    "{distro:?} {channel}"
                );
                assert!(host.commands.lock().unwrap().is_empty());
            }
        }
    }

    #[test]
    fn reinstalling_the_selected_edition_does_not_remove_it() {
        for distro in [
            Distribution::Arch,
            Distribution::Fedora,
            Distribution::Ubuntu,
        ] {
            for channel in ["stable", "development"] {
                let plan = InstallPlan::new(distro, channel).unwrap();
                let host = FakeHost::new(distro, &[plan.package]);
                install_linux_with_host(channel, false, None, &|_| {}, &host).unwrap();
                let commands = host.commands.lock().unwrap();
                assert!(
                    commands
                        .iter()
                        .all(|command| !command.iter().any(|arg| arg == "remove" || arg == "-R")),
                    "{distro:?} {channel}"
                );
                assert!(
                    host.installed
                        .lock()
                        .unwrap()
                        .contains(&plan.package.to_owned())
                );
            }
        }
    }

    #[test]
    fn conflict_preflight_reports_query_failures_without_running_commands() {
        let mut host = FakeHost::new(Distribution::Ubuntu, &[]);
        host.query_error_on = Some("mayaflux".to_owned());
        let error = check_linux_conflict_with_host("development", &host)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Could not query mayaflux"));
        assert!(host.commands.lock().unwrap().is_empty());
    }

    #[test]
    fn conflict_refusal_runs_no_mutating_commands() {
        let host = FakeHost::new(Distribution::Ubuntu, &["mayaflux"]);
        let result = install_linux_with_host("development", false, None, &|_| {}, &host);
        assert!(matches!(result, Err(InstallError::Conflict { .. })));
        assert!(host.commands.lock().unwrap().is_empty());
    }

    #[test]
    fn conflict_preflight_reports_existing_edition_without_mutation() {
        let host = FakeHost::new(Distribution::Ubuntu, &["mayaflux"]);
        assert_eq!(
            check_linux_conflict_with_host("development", &host).unwrap(),
            Some("mayaflux".to_owned())
        );
        assert!(host.commands.lock().unwrap().is_empty());
        assert_eq!(
            check_linux_conflict_with_host("stable", &host).unwrap(),
            None
        );
    }

    #[test]
    fn all_arch_conflicts_must_be_approved_together_before_any_removal() {
        let host = FakeHost::new(Distribution::Arch, &["mayaflux", "mayaflux-bin"]);
        assert_eq!(
            check_linux_conflict_with_host("development", &host).unwrap(),
            Some("mayaflux, mayaflux-bin".to_owned())
        );
        assert!(matches!(
            install_linux_with_host("development", false, None, &|_| {}, &host),
            Err(InstallError::Conflict { .. })
        ));
        assert!(host.commands.lock().unwrap().is_empty());
    }

    #[test]
    fn missing_required_host_tool_stops_before_removing_existing_edition() {
        struct MissingGit(FakeHost);
        impl InstallHost for MissingGit {
            fn available(&self, name: &str) -> bool {
                name != "git" && self.0.available(name)
            }
            fn installed(&self, distro: Distribution, package: &str) -> Result<bool, String> {
                self.0.installed(distro, package)
            }
            fn execute(
                &self,
                command: &[String],
                password: Option<&str>,
                log: &(impl Fn(&str) + Sync),
            ) -> Result<(), String> {
                self.0.execute(command, password, log)
            }
        }
        let host = MissingGit(FakeHost::new(Distribution::Arch, &["mayaflux-dev-bin"]));
        let error = install_linux_with_host("stable", true, None, &|_| {}, &host)
            .unwrap_err()
            .to_string();
        assert!(error.contains("git"));
        assert!(host.0.commands.lock().unwrap().is_empty());
    }

    #[test]
    fn approved_channel_switch_removes_conflict_before_repository_changes() {
        let host = FakeHost::new(Distribution::Ubuntu, &["mayaflux"]);
        install_linux_with_host("development", true, None, &|_| {}, &host).unwrap();
        let commands = host.commands.lock().unwrap();
        assert_eq!(commands[0], ["sudo", "apt-get", "remove", "-y", "mayaflux"]);
        assert_eq!(
            commands[1],
            [
                "sudo",
                "add-apt-repository",
                "-y",
                "ppa:mayaflux/mayaflux-dev"
            ]
        );
        assert_eq!(
            commands.last().unwrap(),
            &["sudo", "apt-get", "install", "-y", "mayaflux-edge"]
        );
    }

    #[test]
    fn arch_clone_does_not_receive_sudo_password_but_makepkg_can_request_it() {
        let host = FakeHost::new(Distribution::Arch, &["mayaflux-bin"]);
        install_linux_with_host("stable", false, Some("secret"), &|_| {}, &host).unwrap();
        assert_eq!(*host.password_use.lock().unwrap(), [false, true]);
    }

    #[test]
    fn failure_after_conflict_removal_reports_recovery_and_stops() {
        let mut host = FakeHost::new(Distribution::Fedora, &["mayaflux-dev"]);
        host.fail_on = Some("copr enable".to_owned());
        let error = install_linux_with_host("stable", true, None, &|_| {}, &host)
            .unwrap_err()
            .to_string();
        assert!(error.contains("mayaflux-dev"));
        assert!(error.contains("restore"));
        assert_eq!(host.commands.lock().unwrap().len(), 2);
    }

    #[test]
    fn failed_repository_setup_preserves_existing_edition_and_can_be_retried() {
        let mut host = FakeHost::new(Distribution::Fedora, &[]);
        host.fail_on = Some("spirv-cross".to_owned());
        let first = install_linux_with_host("stable", false, None, &|_| {}, &host)
            .unwrap_err()
            .to_string();
        assert!(first.contains("spirv-cross"));
        assert_eq!(host.commands.lock().unwrap().len(), 1);
        assert!(host.installed.lock().unwrap().is_empty());
        host.fail_on = None;
        install_linux_with_host("stable", false, None, &|_| {}, &host).unwrap();
        assert!(
            host.installed
                .lock()
                .unwrap()
                .contains(&"mayaflux".to_owned())
        );
    }

    #[test]
    fn failed_final_query_after_removal_reports_recovery() {
        let mut host = FakeHost::new(Distribution::Ubuntu, &["mayaflux"]);
        host.query_error_on = Some("mayaflux-edge".to_owned());
        let error = install_linux_with_host("development", true, None, &|_| {}, &host)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Could not query mayaflux-edge"));
        assert!(error.contains("mayaflux"));
        assert!(error.contains("restore"));
    }

    #[test]
    fn arch_checkout_is_removed_after_success_and_build_failure() {
        for fail_build in [false, true] {
            let mut host = FakeHost::new(Distribution::Arch, &["mayaflux-bin"]);
            if fail_build {
                host.fail_on = Some("makepkg".to_owned());
            }
            let result = install_linux_with_host("stable", false, None, &|_| {}, &host);
            assert_eq!(result.is_err(), fail_build);
            let commands = host.commands.lock().unwrap();
            let checkout = Path::new(commands[0].last().unwrap());
            assert_eq!(commands[1][2], checkout.to_str().unwrap());
            assert!(!checkout.exists(), "checkout left after {fail_build:?}");
        }
    }

    #[test]
    fn failed_arch_clone_cleans_checkout_and_does_not_start_build() {
        let mut host = FakeHost::new(Distribution::Arch, &[]);
        host.fail_on = Some("git clone".to_owned());
        let error = install_linux_with_host("stable", false, None, &|_| {}, &host)
            .unwrap_err()
            .to_string();
        assert!(error.contains("git clone"));
        let commands = host.commands.lock().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(!Path::new(commands[0].last().unwrap()).exists());
    }

    #[test]
    fn second_arch_conflict_removal_failure_reports_first_removed_package() {
        let mut host = FakeHost::new(Distribution::Arch, &["mayaflux", "mayaflux-bin"]);
        host.fail_on = Some("mayaflux-bin".to_owned());
        let error = install_linux_with_host("development", true, None, &|_| {}, &host)
            .unwrap_err()
            .to_string();
        assert!(error.contains("mayaflux"));
        assert!(error.contains("restore"));
        assert_eq!(host.commands.lock().unwrap().len(), 2);
    }

    #[test]
    fn installer_rejects_unverified_package_success() {
        struct MissingInstall(FakeHost);
        impl InstallHost for MissingInstall {
            fn available(&self, name: &str) -> bool {
                self.0.available(name)
            }
            fn installed(&self, distro: Distribution, package: &str) -> Result<bool, String> {
                self.0.installed(distro, package)
            }
            fn execute(
                &self,
                command: &[String],
                password: Option<&str>,
                log: &(impl Fn(&str) + Sync),
            ) -> Result<(), String> {
                if command.iter().any(|arg| arg == "install") {
                    return Ok(());
                }
                self.0.execute(command, password, log)
            }
        }
        let host = MissingInstall(FakeHost::new(Distribution::Ubuntu, &[]));
        let error = install_linux_with_host("stable", false, None, &|_| {}, &host)
            .unwrap_err()
            .to_string();
        assert!(error.contains("not listed as installed"));

        let host = MissingInstall(FakeHost::new(Distribution::Ubuntu, &["mayaflux"]));
        let error = install_linux_with_host("development", true, None, &|_| {}, &host)
            .unwrap_err()
            .to_string();
        assert!(error.contains("not listed as installed"));
        assert!(error.contains("restore"));
    }

    #[test]
    fn gui_command_answers_password_prompt_without_logging_secret() {
        let lines = Mutex::new(Vec::new());
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "printf 'sudo password: '; read -r answer; test \"$answer\" = secret; echo installed"
                .to_owned(),
        ];
        run_linux_command(&command, Some("secret"), &|line| {
            lines.lock().unwrap().push(line.to_owned())
        })
        .unwrap();
        let output = lines.lock().unwrap().join("\n");
        assert!(output.contains("installed"));
        assert!(!output.contains("secret"));
    }

    #[test]
    fn gui_command_answers_fixed_sudo_prompt_without_logging_secret() {
        let lines = Mutex::new(Vec::new());
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "printf '%s' \"$SUDO_PROMPT\"; read -r answer; test \"$answer\" = secret; echo installed"
                .to_owned(),
        ];
        run_linux_command(&command, Some("secret"), &|line| {
            lines.lock().unwrap().push(line.to_owned())
        })
        .unwrap();
        let output = lines.lock().unwrap().join("\n");
        assert!(output.contains("installed"));
        assert!(!output.contains("secret"));
    }

    #[test]
    fn gui_command_confirms_legacy_package_prompt() {
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "printf 'Continue? [Y/n]: '; read -r answer; test \"$answer\" = y".to_owned(),
        ];
        run_linux_command(&command, Some("secret"), &|_| {}).unwrap();
    }

    #[test]
    fn cli_command_stops_after_download_progress_stalls() {
        let command = vec!["sh".to_owned(), "-c".to_owned(), "i=0; while [ $i -lt 50 ]; do printf '[#2089b0 1.0MiB/84.0MiB CN:4 DL:0B]\\r'; i=$((i+1)); sleep 0.02; done".to_owned()];
        let started = Instant::now();
        let error = run_linux_command_with_limits(
            &command,
            None,
            &|_| {},
            Duration::from_millis(120),
            Duration::from_secs(2),
        )
        .unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(error.contains("download progress"));
    }

    #[test]
    fn stalled_transfer_is_not_kept_alive_by_summary_lines() {
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "i=0; while [ $i -lt 30 ]; do printf '[#2089b0 1.0MiB/84.0MiB CN:4 DL:0B]\\r'; printf 'Download Progress Summary\\n'; i=$((i+1)); sleep 0.03; done".to_owned(),
        ];
        for password in [None, Some("secret")] {
            let error = run_linux_command_with_limits(
                &command,
                password,
                &|_| {},
                Duration::from_millis(150),
                Duration::from_secs(2),
            )
            .unwrap_err();
            assert!(error.contains("download progress"));
        }
    }

    #[test]
    fn advancing_transfer_bytes_allow_a_long_download() {
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "i=1; while [ $i -le 8 ]; do printf '[#2089b0 %s.0MiB/84.0MiB CN:4 DL:1MiB]\\r' \"$i\"; i=$((i+1)); sleep 0.08; done".to_owned(),
        ];
        for password in [None, Some("secret")] {
            run_linux_command_with_limits(
                &command,
                password,
                &|_| {},
                Duration::from_millis(250),
                Duration::from_secs(2),
            )
            .unwrap();
        }
    }

    #[test]
    fn silent_commands_time_out_with_and_without_password() {
        let command = vec!["sh".to_owned(), "-c".to_owned(), "sleep 5".to_owned()];
        for password in [None, Some("secret")] {
            let started = Instant::now();
            let error = run_linux_command_with_limits(
                &command,
                password,
                &|_| {},
                Duration::from_secs(2),
                Duration::from_millis(150),
            )
            .unwrap_err();
            assert!(error.contains("No output") || error.contains("No installer output"));
            assert!(started.elapsed() < Duration::from_secs(2));
        }
    }

    #[test]
    fn command_exit_failure_is_reported_after_draining_stderr() {
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "printf 'repository unavailable\\n' >&2; exit 17".to_owned(),
        ];
        for password in [None, Some("secret")] {
            let lines = Mutex::new(Vec::new());
            let error = run_linux_command_with_limits(
                &command,
                password,
                &|line| lines.lock().unwrap().push(line.to_owned()),
                Duration::from_secs(2),
                Duration::from_secs(2),
            )
            .unwrap_err();
            assert!(error.contains("failed") || error.contains("exit"));
            assert!(
                lines
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|line| line == "repository unavailable")
            );
        }
    }

    #[test]
    fn stalled_download_is_stopped_even_when_status_lines_repeat() {
        let command = vec!["sh".to_owned(), "-c".to_owned(), "i=0; while [ $i -lt 50 ]; do printf '[#2089b0 1.0MiB/84.0MiB CN:4 DL:0B]\\r'; i=$((i+1)); sleep 0.02; done".to_owned()];
        let lines = Mutex::new(Vec::new());
        let started = Instant::now();
        let error = run_linux_command_with_limits(
            &command,
            Some("secret"),
            &|line| lines.lock().unwrap().push(line.to_owned()),
            Duration::from_millis(120),
            Duration::from_secs(2),
        )
        .unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(error.contains("download progress"));
    }

    #[test]
    fn installed_query_requires_exact_arch_package_and_ubuntu_installed_status() {
        assert!(!query_output_is_installed(
            Distribution::Arch,
            "mayaflux",
            "mayaflux-bin\n"
        ));
        assert!(query_output_is_installed(
            Distribution::Arch,
            "mayaflux-bin",
            "mayaflux-bin\n"
        ));
        assert!(!query_output_is_installed(
            Distribution::Ubuntu,
            "mayaflux",
            "deinstall ok config-files"
        ));
        assert!(query_output_is_installed(
            Distribution::Ubuntu,
            "mayaflux",
            "install ok installed"
        ));
    }

    #[test]
    fn distro_probe_ignores_non_executable_files() {
        let root =
            std::env::temp_dir().join(format!("weave-executable-check-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("pacman");
        std::fs::write(&file, b"not executable").unwrap();
        assert!(!is_executable(&file));
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_dir(root);
    }

    #[test]
    fn pty_drains_all_output_after_fast_process_exit() {
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "i=0; while [ $i -lt 600 ]; do printf 'line-%s\\n' \"$i\"; i=$((i+1)); done".to_owned(),
        ];
        let lines = Mutex::new(Vec::new());
        run_linux_command(&command, Some("unused"), &|line| {
            lines.lock().unwrap().push(line.to_owned())
        })
        .unwrap();
        let lines = lines.lock().unwrap();
        assert!(lines.iter().any(|line| line == "line-599"));
    }

    #[test]
    fn pty_logs_colored_package_output_without_terminal_escape_codes() {
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "printf '\\033[35mInstalling MayaFlux\\033[0m\\n'".to_owned(),
        ];
        let lines = Mutex::new(Vec::new());
        run_linux_command(&command, Some("unused"), &|line| {
            lines.lock().unwrap().push(line.to_owned())
        })
        .unwrap();
        assert_eq!(*lines.lock().unwrap(), vec!["Installing MayaFlux"]);
    }
}
