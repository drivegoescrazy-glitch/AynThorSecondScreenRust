//! The loopback socket the companion app talks to.
//!
//! Bound to `127.0.0.1:48291`, newline-delimited JSON out, newline-delimited
//! plain-text commands in. No handshake beyond the `hello` line, no auth: the
//! port never leaves the device.
//!
//! The bind retries rather than giving up. A leftover
//! `adb reverse tcp:48291 tcp:48291` leaves adbd owning the port, and the app
//! then connects to adbd, which accepts and instantly drops every connection --
//! which reads on the second screen as a broken app, not as a port conflict.
//! Retrying means clearing the conflict is enough on its own.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::protocol;

pub const PORT: u16 = 48291;
const BIND_RETRY: Duration = Duration::from_secs(5);

pub type Client = Arc<Mutex<TcpStream>>;

pub struct Server {
    clients: Mutex<Vec<Client>>,
    new_clients: Mutex<Vec<Client>>,
    commands: Mutex<VecDeque<String>>,
    bind_failure: Mutex<Option<String>>,
}

static SERVER: OnceLock<&'static Server> = OnceLock::new();

/// The process-wide server, started on first use.
pub fn server() -> &'static Server {
    SERVER.get_or_init(|| {
        let server: &'static Server = Box::leak(Box::new(Server::new()));
        thread::spawn(move || server.accept_loop());
        server
    })
}

impl Server {
    fn new() -> Self {
        Self {
            clients: Mutex::new(Vec::new()),
            new_clients: Mutex::new(Vec::new()),
            commands: Mutex::new(VecDeque::new()),
            bind_failure: Mutex::new(None),
        }
    }

    fn accept_loop(&'static self) {
        loop {
            match TcpListener::bind(("127.0.0.1", PORT)) {
                Ok(listener) => {
                    *self.bind_failure.lock().unwrap() = None;
                    for stream in listener.incoming() {
                        match stream {
                            Ok(stream) => self.add(stream),
                            Err(_) => break,
                        }
                    }
                }
                Err(error) => {
                    *self.bind_failure.lock().unwrap() = Some(error.to_string());
                    thread::sleep(BIND_RETRY);
                }
            }
        }
    }

    fn add(&'static self, stream: TcpStream) {
        let _ = stream.set_nodelay(true);
        let client: Client = Arc::new(Mutex::new(stream));

        // The hello goes out from this thread, before the client is visible to
        // the tick loop, so it is always the first line on the connection.
        write_line(&client, &protocol::hello());

        self.clients.lock().unwrap().push(client.clone());
        self.new_clients.lock().unwrap().push(client.clone());

        thread::spawn(move || {
            let reader = match client.lock().unwrap().try_clone() {
                Ok(stream) => BufReader::new(stream),
                Err(_) => return,
            };
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // The queue is shared with the tick loop; the reader never
                // touches game state itself.
                if let Some(server) = SERVER.get() {
                    server.commands.lock().unwrap().push_back(trimmed.to_string());
                }
            }
        });
    }

    pub fn broadcast(&self, line: &str) {
        let clients: Vec<Client> = self.clients.lock().unwrap().clone();
        for client in clients {
            if !write_line(&client, line) {
                self.drop_client(&client);
            }
        }
    }

    pub fn send_to(&self, client: &Client, line: &str) {
        if !write_line(client, line) {
            self.drop_client(client);
        }
    }

    pub fn has_clients(&self) -> bool {
        !self.clients.lock().unwrap().is_empty()
    }

    pub fn poll_command(&self) -> Option<String> {
        self.commands.lock().unwrap().pop_front()
    }

    pub fn poll_new_client(&self) -> Option<Client> {
        self.new_clients.lock().unwrap().pop()
    }

    pub fn bind_failure(&self) -> Option<String> {
        self.bind_failure.lock().unwrap().clone()
    }

    fn drop_client(&self, client: &Client) {
        self.clients.lock().unwrap().retain(|other| !Arc::ptr_eq(other, client));
        let _ = client.lock().unwrap().shutdown(std::net::Shutdown::Both);
    }
}

/// Writes one line. False when the client is gone and should be dropped.
fn write_line(client: &Client, line: &str) -> bool {
    let Ok(mut stream) = client.lock() else {
        return false;
    };
    let mut payload = Vec::with_capacity(line.len() + 1);
    payload.extend_from_slice(line.as_bytes());
    payload.push(b'\n');
    stream.write_all(&payload).and_then(|_| stream.flush()).is_ok()
}
