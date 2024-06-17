use std::{
    process,
    sync::{Arc, Mutex},
};

use log::error;
use nix::{
    sys::signal::{kill, Signal::SIGTERM},
    unistd::Pid,
};
use once_cell::sync::Lazy;

static PID_LIST: Lazy<Arc<Mutex<Vec<i32>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));

/// Initialize Ctrl-C handler. This should be done at the start
/// of a binary.
///
/// # Panics
/// Will panic if initialized more than once.
pub fn init() {
    let pid_list = PID_LIST.clone();
    ctrlc::set_handler(move || {
        let pid_list = pid_list.lock().expect("Should lock mutex");
        pid_list.iter().for_each(|pid| {
            if let Err(e) = kill(Pid::from_raw(*pid), SIGTERM) {
                error!("Failed to kill process {pid}: Error {e}");
            }
        });
        drop(pid_list);
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
