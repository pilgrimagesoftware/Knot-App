//! A server started as a process and spoken to over its pipes, one JSON
//! message per line.
//!
//! The process is started in a group of its own and the whole group is
//! stopped afterwards. `npx` and `uvx` start the real server as a child of
//! their own, and stopping only the launcher would leave that child running
//! and holding the pipe this side reads.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Instant;

use serde_json::Value;

use crate::endpoint::StdioServer;
use crate::error::{ClientError, Result};
use crate::session::{Exchange, answers};

pub(crate) struct StdioExchange {
    child:    Child,
    stdin:    ChildStdin,
    /// Lines read off the server's stdout by a thread of their own, so a
    /// read can be bounded by the deadline.
    lines:    Receiver<String>,
    deadline: Instant,
}

impl StdioExchange {
    /// Starts `server` with `path` as its `PATH`, stderr discarded.
    pub(crate) fn spawn(server: &StdioServer, path: &str, deadline: Instant) -> Result<Self> {
        let mut command = Command::new(&server.program);
        command.args(&server.args)
               .env("PATH", path)
               .envs(server.env.iter().map(|(name, value)| (name, value)))
               .stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::null());
        if let Some(cwd) = &server.cwd {
            command.current_dir(cwd);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child =
            command.spawn()
                   .map_err(|source| ClientError::Spawn { program: server.program.clone(),
                                                          source })?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take())
        else {
            stop(&mut child);
            return Err(ClientError::Closed);
        };
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line
                else {
                    break;
                };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Self { child,
                  stdin,
                  lines,
                  deadline })
    }

    fn write(&mut self, message: &Value) -> Result<()> {
        let mut line = message.to_string();
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.flush())
            .map_err(|_| ClientError::Closed)
    }
}

impl Exchange for StdioExchange {
    fn request(&mut self, id: u64, message: &Value) -> Result<Value> {
        self.write(message)?;
        loop {
            let left = self.deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) => {
                    // Anything that is not JSON - a banner a server printed
                    // to stdout by mistake - is skipped, not fatal.
                    if let Ok(message) = serde_json::from_str::<Value>(&line)
                       && answers(&message, id)
                    {
                        return Ok(message);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    return Err(ClientError::TimedOut);
                }
                Err(RecvTimeoutError::Disconnected) => return Err(ClientError::Closed),
            }
        }
    }

    fn notify(&mut self, message: &Value) -> Result<()> {
        self.write(message)
    }
}

impl Drop for StdioExchange {
    fn drop(&mut self) {
        stop(&mut self.child);
    }
}

/// Stops the server and everything it started, and reaps it.
fn stop(child: &mut Child) {
    #[cfg(unix)]
    knot_processes::shell::signal_group(child.id(), knot_processes::shell::Signal::Kill);
    let _ = child.kill();
    let _ = child.wait();
}
