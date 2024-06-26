use std::{
    process,
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
};

use log::{error, trace, warn};
use nix::{
    libc::{SIGABRT, SIGCONT, SIGHUP, SIGTSTP},
    sys::signal::{kill, Signal},
    unistd::Pid,
};
use once_cell::sync::Lazy;
use signal_hook::{
    consts::TERM_SIGNALS,
    flag,
    iterator::{exfiltrator::WithOrigin, SignalsInfo},
    low_level,
};

use crate::logging::Logger;

static PID_LIST: Lazy<Arc<Mutex<Vec<i32>>>> = Lazy::new(|| Arc::new(Mutex::new(vec![])));

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

    thread::spawn(app_exec);

    let mut has_terminal = true;
    for info in &mut signals {
        match info.signal {
            termsig if TERM_SIGNALS.contains(&termsig) => {
                warn!("Received termination signal, cleaning up...");
                trace!("{info:#?}");

                Logger::multi_progress().clear().unwrap();

                send_signal_processes(termsig);

                process::exit(1);
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

fn send_signal_processes(sig: i32) {
    let pid_list = PID_LIST.clone();
    let pid_list = pid_list.lock().expect("Should lock mutex");
    pid_list.iter().for_each(|pid| {
        if let Err(e) = kill(Pid::from_raw(*pid), Signal::try_from(sig).unwrap()) {
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
