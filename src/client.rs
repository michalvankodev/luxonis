use client_state::ClientState;
use connection::handle_stream;
use log::{error, info};
use protocol::{ClientMessage, ServerMessage};
use std::{env, path::Path, process};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader, Stdin},
    net::{TcpStream, UnixStream},
    select, signal,
    sync::mpsc::{self, Sender},
};
mod client_state;
mod connection;
mod protocol;
mod validation;

pub enum ClientConnection {
    Tcp(TcpStream),
    Unix(UnixStream),
}
/***
  Client for "guess a word" game
*/
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <TCP URL or .sock path>", args[0]);
        process::exit(1);
    }

    let input = &args[1];
    // let mut client_state = Arc::new(Mutex::new(ClientState::default()));
    let mut client_state = ClientState::default();
    let (tx, mut rx) = mpsc::channel(100);
    let connection = create_connection(input).await?;
    let server_tx = handle_server_connection(connection, tx).await?;

    info!("Connection successful");
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("Failed to register SIGTERM handler");
    let mut user_input = get_user_input_stream();

    loop {
        let previous_status = client_state.status.clone();
        select! {
            server_msg = rx.recv() => {
                match server_msg {
                    Some(msg) =>  client_state.update_from_server(msg),
                    None => {
                        error!("Server disconnected");
                        break;
                    }
                };
            }
            input = user_input.next_line() => {
                let input = input.unwrap().unwrap();
                if let Some(msg) = client_state.update_from_user(&input) {
                    server_tx.send(msg).await?;
                }
            }
            _ = signal::ctrl_c() => {
                break;
            }
            _ = terminate.recv() => {
                break;
            }
        }
        // React to state changes
        if !client_state.status.eq(&previous_status) {
            if let Some(msg) = client_state.process() {
                server_tx.send(msg).await?;
            }
        }
    }

    info!("Gracefully shutting down luxonis game client");
    Ok(())
}

async fn handle_server_connection(
    connection: ClientConnection,
    output_tx: Sender<ServerMessage>,
) -> Result<Sender<ClientMessage>, anyhow::Error> {
    match connection {
        ClientConnection::Tcp(stream) => handle_stream(stream, output_tx).await,
        ClientConnection::Unix(stream) => handle_stream(stream, output_tx).await,
    }
}

async fn create_connection(input: &str) -> Result<ClientConnection, anyhow::Error> {
    if is_valid_sock_path(input) {
        // If it's a Unix socket path
        info!("Attempting to connect to Unix socket: {}", input);
        let unix_stream = UnixStream::connect(input).await?;
        Ok(ClientConnection::Unix(unix_stream))
    } else {
        info!("Attempting to connect to TCP address: {}", input);
        let tcp_stream = TcpStream::connect(input).await?;
        Ok(ClientConnection::Tcp(tcp_stream))
    }
}

fn is_valid_sock_path(path: &str) -> bool {
    let path = Path::new(path);
    path.exists() && path.extension().map_or(false, |ext| ext == "sock")
}

fn get_user_input_stream() -> tokio::io::Lines<BufReader<Stdin>> {
    let stdin = stdin();
    let reader = BufReader::new(stdin);
    reader.lines()
}

// TODO Documentation
// TODO readme documentation
