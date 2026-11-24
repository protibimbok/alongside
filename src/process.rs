//! Spawning commands inside pseudo-terminals.

use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, native_pty_system};

use crate::app::{Id, Msg};
use crate::size::PaneSize;

/// How long commands get to exit after being asked to, before being killed.
const GRACE: Duration = Duration::from_secs(3);

/// A command running in its own PTY.
///
/// Output is sent to the main loop as [`Msg::Output`] by a reader thread, and
/// a waiter thread sends [`Msg::Exited`] once the child is gone.
pub struct Proc {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    pid: Option<u32>,
    exited: Arc<AtomicBool>,
}

impl Proc {
    pub fn spawn(id: Id, command: &str, size: PaneSize, tx: Sender<Msg>) -> Result<Self> {
        let pair = native_pty_system()
            .openpty(size.into())
            .context("failed to open a pty")?;
        let mut child = pair
            .slave
            .spawn_command(shell_command(command))
            .with_context(|| format!("failed to spawn `{command}`"))?;
        // Only the child should hold the slave end, so reads hit EOF when it exits.
        drop(pair.slave);

        let killer = child.clone_killer();
        let pid = child.process_id();
        let exited = Arc::new(AtomicBool::new(false));
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        let out_tx = tx.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if out_tx.send(Msg::Output(id, buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let exited_flag = exited.clone();
        thread::spawn(move || {
            let code = child.wait().map(|status| status.exit_code()).unwrap_or(1);
            exited_flag.store(true, Ordering::SeqCst);
            let _ = tx.send(Msg::Exited(id, code));
        });

        Ok(Self {
            master: pair.master,
            writer,
            killer,
            pid,
            exited,
        })
    }

    /// Sends input to the command, as if typed into its terminal.
    pub fn write(&mut self, bytes: &[u8]) {
        let _ = self
            .writer
            .write_all(bytes)
            .and_then(|()| self.writer.flush());
    }

    pub fn resize(&self, size: PaneSize) {
        let _ = self.master.resize(size.into());
    }

    pub fn is_running(&self) -> bool {
        !self.exited.load(Ordering::SeqCst)
    }

    /// Asks the command and everything it started to exit.
    fn terminate(&mut self) {
        let Some(pid) = self.pid else {
            return self.kill();
        };
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as libc::pid_t), libc::SIGTERM);
        }
        // windows does not have grouping
        #[cfg(windows)]
        {
            let killed = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            if !killed.is_ok_and(|status| status.success()) {
                self.kill();
            }
        }
    }

    /// Forcefully kills the command (and on Unix its process group).
    fn kill(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.pid {
            unsafe {
                libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
            }
        }
        let _ = self.killer.kill();
    }
}

/// A command never outlives its `Proc`, so an early return can't leave one
/// running in the background.
impl Drop for Proc {
    fn drop(&mut self) {
        stop_all([self]);
    }
}

/// Stops every running command: terminate, wait up to [`GRACE`], then kill.
pub fn stop_all<'a>(procs: impl IntoIterator<Item = &'a mut Proc>) {
    let mut running: Vec<_> = procs.into_iter().filter(|p| p.is_running()).collect();
    for proc in &mut running {
        proc.terminate();
    }
    let deadline = Instant::now() + GRACE;
    while running.iter().any(|p| p.is_running()) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    for proc in running {
        if proc.is_running() {
            proc.kill();
        }
    }
}

fn shell_command(command: &str) -> CommandBuilder {
    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = CommandBuilder::new("cmd");
        cmd.args(["/C", command]);
        cmd
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut cmd = CommandBuilder::new("sh");
        cmd.args(["-c", command]);
        cmd.env("TERM", "xterm-256color");
        cmd
    };
    if let Ok(dir) = std::env::current_dir() {
        cmd.cwd(dir);
    }
    cmd
}
