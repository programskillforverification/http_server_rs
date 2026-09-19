mod listener;

use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use crossbeam::channel::{Receiver, unbounded};
use log::{debug, error, info, warn};
use mio::{Events, Interest, Poll, Token};

use crate::error::{ConnError, Error, Result, report};

const THREAD_POOL_SIZE: usize = 4;

pub struct Server {
    host: String,
    port: u16,
}

/// What to do with a connection after handling an event.
enum ConnState {
    Open,
    Closed,
}

impl Server {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    pub fn run(&self) -> Result<()> {
        // --- socket setup ---
        let listener =
            TcpListener::bind((self.host.as_str(), self.port)).map_err(|source| Error::Bind {
                addr: format!("{}:{}", self.host, self.port),
                source,
            })?;
        listener
            .set_nonblocking(true)
            .map_err(Error::ConfigureListener)?;
        info!("Listening on {}:{}", self.host, self.port);

        // --- create channels for each worker ---
        let mut senders = Vec::with_capacity(THREAD_POOL_SIZE);
        for i in 0..THREAD_POOL_SIZE {
            let (tx, rx) = unbounded::<TcpStream>();
            senders.push(tx);
            let poll = Poll::new().map_err(Error::CreatePoll)?;
            thread::Builder::new()
                .name(format!("worker-{i}"))
                .spawn(move || {
                    if let Err(e) = Self::worker_loop(i, poll, rx) {
                        error!("Worker {i} exited: {}", report(&e));
                    }
                })
                .map_err(|source| Error::SpawnWorker { id: i, source })?;
        }

        listener::accept_loop(listener, senders)
    }

    /// Runs a worker's event loop. Errors on a single connection only close
    /// that connection; an `Err` here means the worker itself can't continue.
    pub fn worker_loop(id: usize, mut poll: Poll, rx: Receiver<TcpStream>) -> Result<()> {
        let mut events = Events::with_capacity(1024);
        let mut token_counter: usize = 0;
        let mut connections = HashMap::new();

        info!("Worker {id} started");

        loop {
            // Register new sockets if any
            while let Ok(stream) = rx.try_recv() {
                let token = Token(token_counter);
                token_counter += 1;

                match Self::register(&poll, stream, token) {
                    Ok(stream) => {
                        connections.insert(token.0, stream);
                    }
                    Err(e) => warn!("Worker {id}: dropping connection: {}", report(&e)),
                }
            }

            // poll for events
            match poll.poll(&mut events, Some(Duration::from_millis(100))) {
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                result => result.map_err(Error::Poll)?,
            }

            for event in &events {
                let token = event.token();
                let Some(conn) = connections.get_mut(&token.0) else {
                    continue;
                };
                if !event.is_readable() {
                    continue;
                }

                match Self::handle_readable(id, conn) {
                    Ok(ConnState::Open) => continue,
                    Ok(ConnState::Closed) => debug!("Worker {id}: {token:?} closed by client"),
                    Err(e) => warn!("Worker {id}: {token:?} {}", report(&e)),
                }

                if let Some(mut conn) = connections.remove(&token.0) {
                    // The socket is closed on drop anyway, so a failed deregister is harmless
                    let _ = poll.registry().deregister(&mut conn);
                }
            }
        }
    }

    /// Wraps a new connection for mio and registers it with the worker's poll.
    fn register(
        poll: &Poll,
        stream: TcpStream,
        token: Token,
    ) -> Result<mio::net::TcpStream, ConnError> {
        let mut stream = mio::net::TcpStream::from_std(stream);
        poll.registry()
            .register(&mut stream, token, Interest::READABLE | Interest::WRITABLE)
            .map_err(ConnError::Register)?;
        Ok(stream)
    }

    /// Reads all available data and echoes it back.
    fn handle_readable(id: usize, conn: &mut mio::net::TcpStream) -> Result<ConnState, ConnError> {
        let mut buf = [0u8; 1024];

        // mio is edge-triggered: keep reading until WouldBlock,
        // otherwise leftover data won't trigger another event.
        loop {
            let n = match conn.read(&mut buf) {
                Ok(0) => return Ok(ConnState::Closed),
                Ok(n) => n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(ConnState::Open),
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(ConnError::Read(e)),
            };

            let msg = String::from_utf8_lossy(&buf[..n]);
            debug!("Worker {id}: read {n} bytes: '{msg}'");

            conn.write_all(&buf[..n]).map_err(ConnError::Write)?;
            debug!("Worker {id}: echoed {n} bytes back");
        }
    }
}
