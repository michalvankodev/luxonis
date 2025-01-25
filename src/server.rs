use anyhow::anyhow;
use connection::{send_to_connection, Connection};
use log::{debug, error, info, trace};
use protocol::{ClientMessage, ServerMessage};
use rmp_serde::Serializer;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    fs::remove_file,
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, UnixListener},
    select, signal,
    sync::{
        mpsc::{self, Sender},
        RwLock,
    },
};
use uuid::Uuid;

mod connection;
mod protocol;

const TCP_ADDR: &str = "127.0.0.1:3301";
const UNIX_ADDR: &str = "/tmp/luxonis.sock";

type ActiveConnections = Arc<RwLock<HashMap<Uuid, Arc<RwLock<Connection>>>>>;

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

    let mut active_connections: ActiveConnections =
        Arc::new(RwLock::new(HashMap::<Uuid, Arc<RwLock<Connection>>>::new()));

    let (tx, mut rx) = mpsc::channel(100);

    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("Failed to register SIGTERM handler");

    loop {
        select! {
            // Handle incoming TCP connections.
            tcp_conn = tcp_listener.accept() => {
                match tcp_conn {
                    Ok((stream, addr)) => {
                        debug!("New TCP connection from: {}", addr);
                        let mut connections = active_connections.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let _ = process_socket(Connection::Tcp(stream), tx_clone, &mut connections).await;
                        });
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
                         let mut connections = active_connections.clone();
                         let tx_clone = tx.clone();
                        tokio::spawn(async move {
                        debug!("New Unix socket connection");
                        let _ = process_socket(Connection::Unix(stream), tx_clone, &mut connections).await;
                    });
                    }
                    Err(e) => {
                        error!("Failed to accept Unix socket connection: {}", e);
                    }
                }
            },
            rx_msg = rx.recv() => {
                let mut connections = active_connections.clone();

                tokio::spawn(async move {
                    match rx_msg {
                        Some((player_id, msg)) => {
                          let _ = react_to_client_msg(&player_id, msg, &mut connections).await;
                        }
                        None => {
                            error!("Invalid msg sent to receiver");
                        }
                    }
                });

            }

            _ = signal::ctrl_c() => {
                break;
            }
            _ = terminate.recv() => {
                break;
            }
        }
    }

    info!("Gracefully shutting down luxonis game server");
    drop_all_connections(&mut active_connections).await;
    let _ = remove_file(UNIX_ADDR).await; // Clean up if the file already exists.
}

async fn process_socket(
    connection: Connection,
    tx: Sender<(Uuid, ClientMessage)>,
    active_connections: &mut ActiveConnections,
) -> Result<(), anyhow::Error> {
    debug!("we have a socket");
    // let connections_clone = active_connections.clone();
    let player_id = Uuid::new_v4();
    {
        let mut connections = active_connections.write().await;
        let connection_arc = Arc::new(RwLock::new(connection));
        connections.insert(player_id, connection_arc.clone());
        tokio::spawn(async move {
            let connection = connection_arc.clone();
            loop {
                let mut connection = connection.write().await;
                let msg = read_client_msg(&mut connection).await.unwrap();
                trace!("Message from client parsed: {:?}", msg);
                tx.send((player_id, msg)).await.unwrap();
            }
        });
    }

    send_message(active_connections, &player_id, ServerMessage::AskPassword).await;
    info!("New tcp connection: {}", player_id);
    Ok(())
}

async fn read_client_msg(connection: &mut Connection) -> Result<ClientMessage, anyhow::Error> {
    let mut buf = Vec::<u8>::new();
    match connection {
        Connection::Tcp(ref mut stream) => {
            let mut buf_reader = BufReader::new(stream);
            buf_reader.read_until(b'\0', &mut buf).await?;
        }
        Connection::Unix(ref mut stream) => {
            let mut buf_reader = BufReader::new(stream);
            buf_reader.read_until(b'\0', &mut buf).await?;
        }
    };
    trace!("Message from client received: {:?}", &buf);
    rmp_serde::from_slice::<ClientMessage>(&buf)
        .map_err(|e| anyhow!("Error parsing ServerMessage: {e:?}"))
}

async fn send_message(
    active_connections: &mut ActiveConnections,
    player_id: &Uuid,
    msg: ServerMessage,
) {
    let mut connections = active_connections.write().await;
    let connection = connections.get_mut(player_id).map(|conn| conn.write());

    trace!("Message about to be sent");
    match connection {
        Some(conn) => {
            trace!("Before LOCK {:?}", msg);
            // TODO here we have a deadlock
            let mut conn = conn.await;
            let mut payload = Vec::new();
            msg.serialize(&mut Serializer::new(&mut payload)).unwrap();
            trace!("About to send {:?}", msg);
            send_to_connection(&mut conn, &payload).await;
            trace!("Message sent");
        }
        None => {
            error!("Active connection for {player_id} not found");
        }
    }
}

async fn react_to_client_msg(
    player_id: &Uuid,
    msg: ClientMessage,
    connections: &mut ActiveConnections,
) -> Result<(), anyhow::Error> {
    match msg {
        ClientMessage::AnswerPassword(password) => {
            if password.eq("password") {
                let response = ServerMessage::AssignId(*player_id);
                send_message(connections, player_id, response).await;
            }
        }
        ClientMessage::GetOpponents => todo!(),
        ClientMessage::RequestMatch(_) => todo!(),
        ClientMessage::AcceptMatch(_) => todo!(),
        ClientMessage::DeclineMatch(_) => todo!(),
        ClientMessage::GuessAttempt(_) => todo!(),
        ClientMessage::SendHint(_) => todo!(),
        ClientMessage::GiveUp(_) => todo!(),
    }

    Ok(())
}

async fn drop_all_connections(_active_connections: &mut ActiveConnections) {
    info!("Dropping all active connections");
}

// TODO Respond for password

// TODO Documentation
// TODO readme documentation
