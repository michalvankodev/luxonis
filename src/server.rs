use connection::{handle_stream, Connection};
use log::{debug, error, info, trace};
use protocol::{ClientMessage, ServerMessage};
use rmp_serde::Serializer;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    fs::remove_file,
    io::{AsyncRead, AsyncWrite},
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
                trace!("Received message: {:?}",rx_msg);
                match rx_msg {
                    Some((player_id, msg)) => {
                      let _ = react_to_client_msg(&player_id, msg, &mut connections).await;
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
    drop_all_connections(&mut active_connections).await;
    let _ = remove_file(UNIX_ADDR).await; // Clean up if the file already exists.
}

// Generic client handler for any AsyncRead + AsyncWrite stream
async fn handle_client<S>(
    stream: S,
    main_tx: Sender<(Uuid, ClientMessage)>,
    connections: &mut ActiveConnections,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let player_id = Uuid::new_v4();

    // Create a channel for sending messages to this client
    let (client_tx, mut client_rx) = mpsc::channel::<ClientMessage>(100);

    let client_sender = handle_stream(stream, client_tx).await?;

    tokio::spawn({
        let conns = connections.clone();
        async move {
            // Start receiving messages
            while let Some(msg) = client_rx.recv().await {
                let _ = main_tx.send((player_id, msg)).await;
            }
            // Remove the connection from the shared HashMap
            {
                let mut conns = conns.write().await;
                info!("Connection with {} closed", player_id);
                conns.remove(&player_id);
            }
        }
    });

    {
        let mut conns = connections.write().await;
        conns.insert(
            player_id,
            Connection {
                tx: client_sender.clone(),
            },
        );
    }

    info!("Client connected: {}", player_id);
    let _ = client_sender.send(ServerMessage::AskPassword).await;

    Ok(())
}

async fn send_message(
    active_connections: &mut ActiveConnections,
    player_id: &Uuid,
    msg: ServerMessage,
) -> Result<(), anyhow::Error> {
    let mut connections = active_connections.write().await;
    let connection = connections.get_mut(player_id).unwrap().clone();
    drop(connections);

    trace!("Message about to be sent");
    trace!("Before LOCK {:?}", msg);
    let mut payload = Vec::new();
    msg.serialize(&mut Serializer::new(&mut payload))?;
    trace!("About to send {:?}", msg);
    connection.tx.send(msg).await?;
    trace!("Message sent");
    Ok(())
}

async fn react_to_client_msg(
    player_id: &Uuid,
    msg: ClientMessage,
    connections: &mut ActiveConnections,
) -> Result<(), anyhow::Error> {
    match msg {
        ClientMessage::AnswerPassword(password) => {
            debug!("password attempt");
            if password.eq("password") {
                let response = ServerMessage::AssignId(*player_id);
                send_message(connections, player_id, response).await?;
            }
        }
        ClientMessage::GetOpponents => todo!(),
        ClientMessage::RequestMatch(_) => todo!(),
        ClientMessage::AcceptMatch(_) => todo!(),
        ClientMessage::DeclineMatch(_) => todo!(),
        ClientMessage::GuessAttempt(_) => todo!(),
        ClientMessage::SendHint(_) => todo!(),
        ClientMessage::GiveUp(_) => todo!(),
        ClientMessage::LeaveGame => todo!(),
    }

    Ok(())
}

async fn drop_all_connections(_active_connections: &mut ActiveConnections) {
    info!("Dropping all active connections");
}

// TODO Respond for password

// TODO Documentation
// TODO readme documentation
