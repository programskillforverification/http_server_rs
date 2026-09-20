use std::io;

use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that stop the whole server, or a whole worker.
#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to bind {addr}")]
    Bind { addr: String, source: io::Error },

    #[error("failed to configure listener")]
    ConfigureListener(#[source] io::Error),

    #[error("failed to create poll instance")]
    CreatePoll(#[source] io::Error),

    #[error("failed to spawn worker {id}")]
    SpawnWorker { id: usize, source: io::Error },

    #[error("failed to accept connection")]
    Accept(#[source] io::Error),

    #[error("failed to poll for events")]
    Poll(#[source] io::Error),
}

/// Errors that only affect a single connection: it gets dropped,
/// and the server keeps running.
#[derive(Debug, Error)]
pub enum ConnError {
    #[error("failed to set non-blocking mode")]
    SetNonblocking(#[source] io::Error),

    #[error("worker {0} has shut down")]
    WorkerGone(usize),

    #[error("failed to register with poll")]
    Register(#[source] io::Error),

    #[error("read failed")]
    Read(#[source] io::Error),

    #[error("write failed")]
    Write(#[source] io::Error),
}

/// Why a byte slice could not be parsed as an HTTP response.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("headers are not terminated by a blank line")]
    IncompleteHead,

    #[error("headers are not valid UTF-8")]
    NotUtf8(#[from] std::str::Utf8Error),

    #[error("malformed status line: {0:?}")]
    StatusLine(String),

    #[error("{0} is not a valid status code")]
    StatusCode(u16),

    #[error("malformed header line: {0:?}")]
    HeaderLine(String),
}

/// Formats an error with its full source chain, e.g. `read failed: Connection reset by peer`.
pub(crate) fn report(err: &dyn std::error::Error) -> String {
    let mut msg = err.to_string();
    let mut source = err.source();
    while let Some(e) = source {
        msg.push_str(": ");
        msg.push_str(&e.to_string());
        source = e.source();
    }
    msg
}
