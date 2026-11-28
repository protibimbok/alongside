//! One command and the terminal screen it draws on.

use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::app::{Id, Msg};
use crate::process::{self, Proc};
use crate::size::PaneSize;
use crate::view::View;

const STARTUP: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
pub enum Status {
    Running,
    Exited(u32),
}

pub struct Pane {
    name: String,
    view: View,
    status: Status,
    command: String,
    proc: Proc,
    /// The run this pane is showing; bumped on every restart.
    id: Id,
    started: Instant,
}

impl Pane {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn status(&self) -> Status {
        self.status
    }

    /// Time since the command was spawned, while it is still starting up.
    pub fn starting(&self) -> Option<Duration> {
        let uptime = self.started.elapsed();
        (matches!(self.status, Status::Running) && uptime < STARTUP).then_some(uptime)
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    pub fn spawn(
        index: usize,
        name: String,
        command: String,
        size: PaneSize,
        tx: &Sender<Msg>,
    ) -> Result<Self> {
        let id = Id {
            pane: index,
            run: 0,
        };
        Ok(Self {
            name,
            view: View::new(size),
            status: Status::Running,
            proc: Proc::spawn(id, &command, size, tx.clone())?,
            command,
            id,
            started: Instant::now(),
        })
    }

    /// Stops the command if needed and starts it again on a clean screen.
    pub fn restart(&mut self, size: PaneSize, tx: &Sender<Msg>) {
        process::stop_all([&mut self.proc]);
        self.id.run += 1;
        self.view = View::new(size);
        self.started = Instant::now();
        match Proc::spawn(self.id, &self.command, size, tx.clone()) {
            Ok(proc) => {
                self.proc = proc;
                self.status = Status::Running;
            }
            Err(err) => {
                self.view
                    .process(format!("concurtui: {err:#}\r\n").as_bytes());
                self.status = Status::Exited(1);
            }
        }
    }

    pub fn resize(&mut self, size: PaneSize) {
        self.view.resize(size);
        self.proc.resize(size);
    }

    /// Whether `id` is the run this pane is showing, rather than an earlier
    /// run of a command that was restarted since.
    pub fn is_current(&self, id: Id) -> bool {
        self.id == id
    }

    pub fn set_exited(&mut self, code: u32) {
        self.status = Status::Exited(code);
    }

    /// Sends bytes to the command, as if they were typed.
    pub fn write(&mut self, bytes: &[u8]) {
        self.proc.write(bytes);
    }
}

/// Stops the command of every pane that is still running.
pub fn stop_all(panes: &mut [Pane]) {
    process::stop_all(panes.iter_mut().map(|pane| &mut pane.proc));
}
