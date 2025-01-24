use log::{debug, error, info};

/***
  Server for "guess a word" game
*/
use tokio::{
    join,
    net::{TcpListener, TcpStream, UnixListener, UnixStream},
    select,
};

mod protocol;

const TCP_ADDR: &str = "127.0.0.1:3301";
const UNIX_ADDR: &str = "/tmp/luxonis.sock";

#[tokio::main]
async fn main() {
    env_logger::init();
    // Bind the listener to the address
    let tcp_listener = TcpListener::bind(TCP_ADDR).await.unwrap();
    debug!("TCP listener started at: {TCP_ADDR}");
    let unix_listener = UnixListener::bind(UNIX_ADDR).unwrap();

    loop {
        select! {
        // Handle incoming TCP connections.
        tcp_conn = tcp_listener.accept() => {
            match tcp_conn {
                Ok((stream, addr)) => {
                    debug!("New TCP connection from: {}", addr);
                    tokio::spawn(process_tcp(stream));
                }
                Err(e) => {
                    error!("Failed to accept TCP connection: {}", e);
                }
            }
        }

        // Handle incoming Unix socket connections.
        unix_conn = unix_listener.accept() => {
            match unix_conn {
                Ok((stream, _addr)) => {
                    debug!("New Unix socket connection");
                    tokio::spawn(process_unix(stream));
                }
                Err(e) => {
                    error!("Failed to accept Unix socket connection: {}", e);
                }
            }
        }
        }
    }
}

async fn process_tcp(_socket: TcpStream) {
    debug!("we have a socket");
}

async fn process_unix(_socket: UnixStream) {
    debug!("we have a socket");
}

// TODO Documentation
// TODO readme documentation
