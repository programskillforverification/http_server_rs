mod listener;

use std::thread;
pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    pub fn run(&self) -> std::io::Result<()> {
        println!("Starting server on {}", self.addr);

        thread::scope(|s| {
            s.spawn(|| {
                if let Err(e) = listener::run(&self.addr) {
                    eprintln!("Listener error: {}", e);
                }
            });

            println!("Server is running. Press Ctrl+C to stop.");
        });

        Ok(())
    }
}
