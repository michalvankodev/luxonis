use log::{debug, error, info, trace};
use protocol::ServerMessage;
use rmp_serde::Serializer;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::{
    fs::remove_file,
    net::{TcpListener, TcpStream, UnixListener, UnixStream},
    select, signal,
    sync::RwLock,
};
use uuid::Uuid;

mod protocol;

const TCP_ADDR: &str = "127.0.0.1:3301";
const UNIX_ADDR: &str = "/tmp/luxonis.sock";

enum Connection {
    Tcp(TcpStream),
    Unix(UnixStream),
}

type ActiveConnections = Arc<RwLock<HashMap<Uuid, Connection>>>;

/***
  Server for "guess a word" game
*/
#[tokio::main]
async fn main() {
    env_logger::init();
    // Bind the listener to the address
    let tcp_listener = TcpListener::bind(TCP_ADDR).await.unwrap();
    debug!("TCP listener started at: {TCP_ADDR}");
    let _ = remove_file(UNIX_ADDR).await; // Clean up if the file already exists.
    let unix_listener = UnixListener::bind(UNIX_ADDR).unwrap();
    debug!("TCP listener started at: {UNIX_ADDR}");

    let active_connections: ActiveConnections =
        Arc::new(RwLock::new(HashMap::<Uuid, Connection>::new()));

    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("Failed to register SIGTERM handler");
    loop {
        select! {
            // Handle incoming TCP connections.
            tcp_conn = tcp_listener.accept() => {
                match tcp_conn {
                    Ok((stream, addr)) => {
                        debug!("New TCP connection from: {}", addr);
                        tokio::spawn(process_tcp(stream, active_connections.clone()));
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
                        debug!("New Unix socket connection");
                        tokio::spawn(process_unix(stream, active_connections.clone()));
                    }
                    Err(e) => {
                        error!("Failed to accept Unix socket connection: {}", e);
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
    drop_all_connections(active_connections.clone()).await;
    let _ = remove_file(UNIX_ADDR).await; // Clean up if the file already exists.
}

async fn process_tcp(socket: TcpStream, active_connections: ActiveConnections) {
    debug!("we have a tcp socket");

    let connections_clone = active_connections.clone();
    let mut connections = active_connections.write().await;
    let player_id = Uuid::new_v4();
    connections.insert(player_id, Connection::Tcp(socket));
    info!("New tcp connection: {}", player_id);
    drop(connections);

    send_message(connections_clone, &player_id, ServerMessage::AskPassword).await;

    // if let Err(e) = socket.write_all(b"Welcome brave challanger").await {
    //     let addr = socket
    //         .peer_addr()
    //         .map(|addr| addr.to_string())
    //         .unwrap_or("unknown address".to_string());
    //     error!("Failed to sent message to {addr}, {e:?}");
    // };
}

async fn process_unix(socket: UnixStream, active_connections: ActiveConnections) {
    debug!("we have a unix socket");

    let connections_clone = active_connections.clone();
    let mut connections = active_connections.write().await;
    let player_id = Uuid::new_v4();
    connections.insert(player_id, Connection::Unix(socket));
    info!("New unix connection: {}", player_id);
    drop(connections);

    send_message(connections_clone, &player_id, ServerMessage::AskPassword).await;
}

async fn send_message(active_connections: ActiveConnections, player_id: &Uuid, msg: ServerMessage) {
    let mut connections = active_connections.write().await;
    let connection = connections.get_mut(player_id);

    trace!("Message about to be sent");
    match connection {
        Some(conn) => {
            let mut payload = Vec::new();
            msg.serialize(&mut Serializer::new(&mut payload)).unwrap();
            send_to_connection(conn, &payload).await;
            trace!("Message sent");
        }
        None => {
            error!("Active connection for {player_id} not found");
        }
    }
}

async fn send_to_connection(connection: &mut Connection, payload: &[u8]) {
    match connection {
        Connection::Tcp(stream) => {
            let _ = stream.write_all(payload).await;
        }
        Connection::Unix(stream) => {
            let _ = stream.write_all(payload).await;
        }
    }
}

async fn drop_all_connections(_active_connections: ActiveConnections) {
    info!("Dropping all active connections");
}

// TODO Respond for password

// TODO Documentation
// TODO readme documentation
