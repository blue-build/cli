use std::{
    fs,
    path::PathBuf,
    process::{self, Command},
    sync::{Arc, Mutex},
};

use log::{debug, error};
use nix::{
    sys::signal::{kill, Signal::SIGTERM},
    unistd::Pid,
};
use once_cell::sync::Lazy;

use crate::logging::Logger;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerId {
    cid_path: PathBuf,
    requires_sudo: bool,
    crt: String,
}

impl ContainerId {
    pub fn new<P, S>(cid_path: P, container_runtime: S, requires_sudo: bool) -> Self
    where
        P: Into<PathBuf>,
        S: Into<String>,
    {
        let cid_path = cid_path.into();
        let crt = container_runtime.into();
        Self {
            cid_path,
            requires_sudo,
            crt,
        }
    }
}

static PID_LIST: Lazy<Arc<Mutex<Vec<i32>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));
static CID_LIST: Lazy<Arc<Mutex<Vec<ContainerId>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));

/// Initialize Ctrl-C handler. This should be done at the start
/// of a binary.
///
/// # Panics
/// Will panic if initialized more than once.
pub fn init() {
    let pid_list = PID_LIST.clone();
    let cid_list = CID_LIST.clone();
    ctrlc::set_handler(move || {
        let cid_list = cid_list.lock().expect("Should lock mutex");
        let pid_list = pid_list.lock().expect("Should lock mutex");

        if let Err(e) = Logger::multi_progress().clear() {
            error!("Failed to remove multi progress bar: {e}");
        }

        pid_list.iter().for_each(|pid| {
            debug!("Killing pid {pid}");
            if let Err(e) = kill(Pid::from_raw(*pid), SIGTERM) {
                error!("Failed to kill process {pid}: Error {e}");
            }
        });
        drop(pid_list);

        cid_list.iter().for_each(|cid| {
            if let Ok(id) = fs::read_to_string(&cid.cid_path) {
                let id = id.trim();
                debug!("Killing container {id}");

                if let Err(e) = if cid.requires_sudo {
                    Command::new("sudo")
                        .arg(&cid.crt)
                        .arg("kill")
                        .arg(id)
                        .status()
                } else {
                    Command::new(&cid.crt).arg("kill").arg(id).status()
                } {
                    error!("Failed to kill container {id}: Error {e}");
                }
            }
        });
        drop(cid_list);

        process::exit(1);
    })
    .expect("Should create ctrlc handler");
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
pub fn add_cid(cid: &ContainerId) {
    let mut cid_list = CID_LIST.lock().expect("Should lock cid_list");

    if !cid_list.contains(cid) {
        cid_list.push(cid.clone());
    }
}

/// Remove a cid from the list of pids to kill.
///
/// # Panics
/// Will panic if the mutex cannot be locked.
pub fn remove_cid(cid: &ContainerId) {
    let mut cid_list = CID_LIST.lock().expect("Should lock cid_list");

    if let Some(index) = cid_list.iter().position(|val| *val == *cid) {
        cid_list.swap_remove(index);
    }
}
