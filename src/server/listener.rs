use std::io::ErrorKind;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use crossbeam::channel::Sender;
use log::{info, warn};

use crate::error::{ConnError, Error, Result, report};

/// Accepts connections and hands them to workers round-robin.
/// Only returns if the listener itself fails.
pub fn accept_loop(listener: TcpListener, senders: Vec<Sender<TcpStream>>) -> Result<()> {
    let mut idx: usize = 0;
    loop {
        let (stream, peer) = match listener.accept() {
            Ok(accepted) => accepted,
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            // The client gave up before we accepted it; not our problem
            Err(e) if e.kind() == ErrorKind::ConnectionAborted => continue,
            Err(e) => return Err(Error::Accept(e)),
        };
        info!("Accepted connection from {peer}");

        if let Err(e) = dispatch(stream, &senders[idx], idx) {
            warn!("Dropping connection from {peer}: {}", report(&e));
        }
        idx = (idx + 1) % senders.len();
    }
}

/// Prepares a new connection and sends it to a worker.
fn dispatch(stream: TcpStream, sender: &Sender<TcpStream>, worker: usize) -> Result<(), ConnError> {
    // mio requires non-blocking sockets
    stream
        .set_nonblocking(true)
        .map_err(ConnError::SetNonblocking)?;
    sender
        .send(stream)
        .map_err(|_| ConnError::WorkerGone(worker))?;
    Ok(())
}
