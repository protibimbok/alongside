//! Spawning commands inside pseudo-terminals.

use std::io::Read;
use std::sync::mpsc::Sender;
use std::thread;

use anyhow::{Context, Result};
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::app::Msg;

/// A command running in its own PTY.
///
/// Output is sent to the main loop as [`Msg::Output`] by a reader thread, and
/// a waiter thread sends [`Msg::Exited`] once the child is gone.
pub struct Proc {
    master: Box<dyn MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

impl Proc {
    pub fn spawn(index: usize, command: &str, size: (u16, u16), tx: Sender<Msg>) -> Result<Self> {
        let pair = native_pty_system()
            .openpty(pty_size(size))
            .context("failed to open a pty")?;
        let mut child = pair
            .slave
            .spawn_command(shell_command(command))
            .with_context(|| format!("failed to spawn `{command}`"))?;
        // Only the child should hold the slave end, so reads hit EOF when it exits.
        drop(pair.slave);

        let killer = child.clone_killer();
        let mut reader = pair.master.try_clone_reader()?;

        let out_tx = tx.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if out_tx.send(Msg::Output(index, buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        thread::spawn(move || {
            let code = child.wait().map(|status| status.exit_code()).unwrap_or(1);
            let _ = tx.send(Msg::Exited(index, code));
        });

        Ok(Self {
            master: pair.master,
            killer,
        })
    }

    pub fn resize(&self, size: (u16, u16)) {
        let _ = self.master.resize(pty_size(size));
    }

    pub fn kill(&mut self) {
        let _ = self.killer.kill();
    }
}

fn pty_size((rows, cols): (u16, u16)) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
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
