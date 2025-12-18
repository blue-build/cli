use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, Command, ExitStatus},
    sync::{Arc, Mutex, atomic::AtomicBool},
    thread,
};

use blue_build_utils::{
    constants::SUDO_ASKPASS, container::ContainerId, has_env_var, running_as_root,
};
use comlexr::cmd;
use log::{debug, error, trace, warn};
use miette::{IntoDiagnostic, Result, bail};
use nix::{
    libc::{SIGABRT, SIGCONT, SIGHUP, SIGTSTP},
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use signal_hook::{
    consts::TERM_SIGNALS,
    flag,
    iterator::{SignalsInfo, exfiltrator::WithOrigin},
    low_level,
};

use crate::logging::Logger;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerSignalId {
    cid_path: PathBuf,
    requires_sudo: bool,
    container_runtime: ContainerRuntime,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ContainerRuntime {
    Podman,
    Docker,
}

impl std::fmt::Display for ContainerRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            Self::Podman => "podman",
            Self::Docker => "docker",
        })
    }
}

impl ContainerSignalId {
    pub fn new<P>(cid_path: P, container_runtime: ContainerRuntime, requires_sudo: bool) -> Self
    where
        P: Into<PathBuf>,
    {
        let cid_path = cid_path.into();
        Self {
            cid_path,
            requires_sudo,
            container_runtime,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DetachedContainer {
    signal_id: ContainerSignalId,
    container_id: ContainerId,
}

impl DetachedContainer {
    /// Runs the provided command to start a container with the given signal ID,
    /// taking the output of the command as the container ID.
    ///
    /// # Errors
    /// Returns an error if the command fails or if the output of the command
    /// is invalid UTF-8.
    pub fn start(signal_id: ContainerSignalId, mut cmd: Command) -> Result<Self> {
        trace!("DetachedContainer::start({signal_id:#?}, {cmd:#?})");

        add_cid(&signal_id);

        let output = cmd.output().into_diagnostic()?;
        if !output.status.success() {
            remove_cid(&signal_id);
            bail!(
                "Failed to start detached container image.\nstderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let container_id = {
            let mut stdout = output.stdout;
            while stdout.pop_if(|byte| byte.is_ascii_whitespace()).is_some() {}
            ContainerId(String::from_utf8(stdout).into_diagnostic()?)
        };

        Ok(Self {
            signal_id,
            container_id,
        })
    }

    #[must_use]
    pub const fn id(&self) -> &ContainerId {
        &self.container_id
    }

    #[must_use]
    pub fn cid_path(&self) -> &Path {
        &self.signal_id.cid_path
    }
}

impl Drop for DetachedContainer {
    fn drop(&mut self) {
        if kill_container(&self.signal_id).is_ok_and(|exit_status| exit_status.success()) {
            remove_cid(&self.signal_id);
        }
    }
}

static PID_LIST: std::sync::LazyLock<Arc<Mutex<Vec<i32>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(vec![])));
static CID_LIST: std::sync::LazyLock<Arc<Mutex<Vec<ContainerSignalId>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(vec![])));

/// Initialize Ctrl-C handler. This should be done at the start
/// of a binary.
///
/// # Panics
/// Will panic if initialized more than once.
pub fn init<F>(app_exec: F)
where
    F: FnOnce() + Send + 'static,
{
    // Make sure double CTRL+C and similar kills
    let term_now = Arc::new(AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        // When terminated by a second term signal, exit with exit code 1.
        // This will do nothing the first time (because term_now is false).
        flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term_now))
            .expect("Register conditional shutdown");
        // But this will "arm" the above for the second time, by setting it to true.
        // The order of registering these is important, if you put this one first, it will
        // first arm and then terminate ‒ all in the first round.
        flag::register(*sig, Arc::clone(&term_now)).expect("Register signal");
    }

    let mut signals = vec![SIGABRT, SIGHUP, SIGTSTP, SIGCONT];
    signals.extend(TERM_SIGNALS);
    let mut signals = SignalsInfo::<WithOrigin>::new(signals).expect("Need signal info");

    thread::spawn(|| {
        let app = thread::spawn(app_exec);

        if matches!(app.join(), Ok(())) {
            exit_unwind(0);
        } else {
            error!("App thread panic!");
            exit_unwind(2);
        }
    });

    let mut has_terminal = true;
    for info in &mut signals {
        match info.signal {
            termsig if TERM_SIGNALS.contains(&termsig) => {
                warn!("Received termination signal, cleaning up...");
                trace!("{info:#?}");

                Logger::multi_progress()
                    .clear()
                    .expect("Should clear multi_progress");

                send_signal_processes(termsig);

                let cid_list = CID_LIST.clone();
                let cid_list = cid_list.lock().expect("Should lock mutex");
                cid_list.iter().for_each(|cid| {
                    let _ = kill_container(cid);
                });
                drop(cid_list);

                exit_unwind(1);
            }
            SIGTSTP => {
                if has_terminal {
                    send_signal_processes(SIGTSTP);
                    has_terminal = false;
                    low_level::emulate_default_handler(SIGTSTP).expect("Should stop");
                }
            }
            SIGCONT => {
                if !has_terminal {
                    send_signal_processes(SIGCONT);
                    has_terminal = true;
                }
            }
            _ => {
                trace!("Received signal {info:#?}");
            }
        }
    }
}

struct ExitCode {
    code: i32,
}

impl Drop for ExitCode {
    fn drop(&mut self) {
        process::exit(self.code);
    }
}

fn exit_unwind(code: i32) {
    std::panic::resume_unwind(Box::new(ExitCode { code }));
}

fn send_signal_processes(sig: i32) {
    let pid_list = PID_LIST.clone();
    let pid_list = pid_list.lock().expect("Should lock mutex");

    pid_list.iter().for_each(|pid| {
        if let Err(e) = kill(
            Pid::from_raw(*pid),
            Signal::try_from(sig).expect("Should be valid signal"),
        ) {
            error!("Failed to kill process {pid}: Error {e}");
        } else {
            trace!("Killed process {pid}");
        }
    });
    drop(pid_list);
}

/// Add a pid to the list to kill when the program
/// recieves a kill signal.
///
/// # Panics
/// Will panic if the mutex cannot be locked.
pub fn add_pid<T>(pid: T)
where
    T: TryInto<i32>,
{
    if let Ok(pid) = pid.try_into() {
        let mut pid_list = PID_LIST.lock().expect("Should lock pid_list");

        if !pid_list.contains(&pid) {
            pid_list.push(pid);
        }
    }
}

/// Remove a pid from the list of pids to kill.
///
/// # Panics
/// Will panic if the mutex cannot be locked.
pub fn remove_pid<T>(pid: T)
where
    T: TryInto<i32>,
{
    if let Ok(pid) = pid.try_into() {
        let mut pid_list = PID_LIST.lock().expect("Should lock pid_list");

        if let Some(index) = pid_list.iter().position(|val| *val == pid) {
            pid_list.swap_remove(index);
        }
    }
}

/// Add a cid to the list to kill when the program
/// recieves a kill signal.
///
/// # Panics
/// Will panic if the mutex cannot be locked.
pub fn add_cid(cid: &ContainerSignalId) {
    let mut cid_list = CID_LIST.lock().expect("Should lock cid_list");

    if !cid_list.contains(cid) {
        cid_list.push(cid.clone());
    }
}

/// Remove a cid from the list of pids to kill.
///
/// # Panics
/// Will panic if the mutex cannot be locked.
pub fn remove_cid(cid: &ContainerSignalId) {
    let mut cid_list = CID_LIST.lock().expect("Should lock cid_list");

    if let Some(index) = cid_list.iter().position(|val| *val == *cid) {
        cid_list.swap_remove(index);
    }
}

fn kill_container(cid: &ContainerSignalId) -> Result<ExitStatus> {
    fs::read_to_string(&cid.cid_path)
        .into_diagnostic()
        .and_then(|id| {
            let id = id.trim();
            debug!("Killing container {id}");

            let status = cmd!(
                if cid.requires_sudo && !running_as_root() {
                    "sudo".to_string()
                } else {
                    cid.container_runtime.to_string()
                },
                if cid.requires_sudo && !running_as_root() && has_env_var(SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    format!("Password needed to kill container {id}"),
                ],
                if cid.requires_sudo && !running_as_root() => cid.container_runtime.to_string(),
                "stop",
                id
            )
            .status();

            if let Err(e) = &status {
                error!("Failed to kill container {id}: Error {e}");
            }
            status.into_diagnostic()
        })
}
