use connection::Connection;
use log::{debug, error, info, trace};
use protocol::ServerMessage;
use server_connection::{handle_client, react_to_client_msg};
use server_state::ServerState;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    fs::remove_file,
    net::{TcpListener, UnixListener},
    select, signal,
    sync::{
        mpsc::{self},
        RwLock,
    },
};
use uuid::Uuid;

mod connection;
mod protocol;
mod server_connection;
mod server_state;

const TCP_ADDR: &str = "127.0.0.1:3301";
const UNIX_ADDR: &str = "/tmp/luxonis.sock";

type ActiveConnections = Arc<RwLock<HashMap<Uuid, Connection>>>;

///  Server application for "guess a word" game
#[tokio::main]
async fn main() {
    env_logger::init();
    // Bind the listener to the address
    let tcp_listener = TcpListener::bind(TCP_ADDR).await.unwrap();
    debug!("TCP listener started at: {TCP_ADDR}");
    let _ = remove_file(UNIX_ADDR).await; // Clean up if the file already exists.
    let unix_listener = UnixListener::bind(UNIX_ADDR).unwrap();
    debug!("TCP listener started at: {UNIX_ADDR}");

    let server_state = Arc::new(RwLock::new(ServerState::default()));
    let mut active_connections: ActiveConnections =
        Arc::new(RwLock::new(HashMap::<Uuid, Connection>::new()));

    let (tx, mut rx) = mpsc::channel(100);

    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("Failed to register SIGTERM handler");

    loop {
        select! {
            // Handle incoming TCP connections.
            tcp_conn = tcp_listener.accept() => {
                match tcp_conn {
                    Ok((stream, _addr)) => {
                        // let mut connections = active_connections.clone();
                        let _ = handle_client(stream, tx.clone(), &mut active_connections).await;
                    }
                    Err(e) => {
                        error!("Failed to accept TCP connection: {}", e);
                    }
                }
            },
            // Handle incoming Unix socket connections.
            unix_conn = unix_listener.accept() => {
                match unix_conn {
                    Ok((stream, _addr)) => {
                        // let mut connections = active_connections.clone();
                        let _ = handle_client(stream, tx.clone(), &mut active_connections).await;
                    }

                    Err(e) => {
                        error!("Failed to accept Unix socket connection: {}", e);
                    }
                }
            },
            rx_msg = rx.recv() => {
                let mut connections = active_connections.clone();
                let mut server_state = server_state.write().await;
                trace!("Received message: {:?}",rx_msg);
                match rx_msg {
                    Some((player_id, msg)) => {
                      let _ = react_to_client_msg(&player_id, msg, &mut connections, &mut server_state).await;
                    }
                    None => {
                        error!("Invalid msg sent to receiver");
                    }
                }
            },
            _ = signal::ctrl_c() => {
                break;
            }
            _ = terminate.recv() => {
                break;
            }
        }
    }

    info!("Gracefully shutting down luxonis game server");
    let _ = drop_all_connections(&mut active_connections).await;
    let _ = remove_file(UNIX_ADDR).await; // Clean up if the file already exists.
}

/// Send a disconnect message to all connected players
async fn drop_all_connections(
    active_connections: &mut ActiveConnections,
) -> Result<(), anyhow::Error> {
    for connection in active_connections.write().await.values_mut() {
        connection.tx.send(ServerMessage::Disconnect).await?;
    }
    Ok(())
}

// TODO Documentation
// TODO optimize buffers
