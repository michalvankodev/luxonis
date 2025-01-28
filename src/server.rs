use anyhow::anyhow;
use connection::{handle_stream, Connection};
use log::{debug, error, info, trace};
use protocol::{ClientMessage, ClientRequestError, ServerMessage};
use rmp_serde::Serializer;
use serde::Serialize;
use server_state::{MatchState, ServerState};
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
mod server_state;

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
    let connection = connections
        .get_mut(player_id)
        .ok_or(anyhow!("Player does no longer exists"))?
        .clone();
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
    server_state: &mut ServerState,
) -> Result<(), anyhow::Error> {
    match msg {
        ClientMessage::AnswerPassword(password) => {
            debug!("password attempt");
            if password.eq("password") {
                let response = ServerMessage::AssignId(*player_id);
                server_state.add_available_player(player_id);
                send_message(connections, player_id, response).await?;
            }
        }
        ClientMessage::GetOpponents => {
            let opponents = &server_state
                .available_players
                .clone()
                .into_iter()
                .filter(|player| player.ne(player_id))
                .collect::<Vec<Uuid>>();

            let response = ServerMessage::ListOpponents(opponents.clone());
            send_message(connections, player_id, response).await?;
        }
        ClientMessage::RequestMatch(opponent, guess_word) => {
            if let Some(match_id) =
                server_state.create_new_match((player_id, &opponent), &guess_word)
            {
                send_message(
                    connections,
                    &opponent,
                    ServerMessage::MatchStarted(match_id),
                )
                .await?;
                send_message(
                    connections,
                    player_id,
                    ServerMessage::MatchAccepted(match_id),
                )
                .await?;
            } else {
                send_message(
                    connections,
                    player_id,
                    ServerMessage::BadRequest(ClientRequestError::CannotCreateMatch),
                )
                .await?;
            }
        }
        ClientMessage::GuessAttempt(match_id, guess) => {
            if let Some(active_match) = server_state.active_matches.get_mut(&match_id) {
                active_match.attempt(&guess);

                match active_match.state {
                    MatchState::Active => {
                        send_message(
                            connections,
                            &active_match.challenger,
                            ServerMessage::MatchAttempt(
                                match_id,
                                active_match.attempts,
                                active_match.hints.len() as u32,
                                guess,
                            ),
                        )
                        .await?;
                        send_message(
                            connections,
                            &active_match.guesser,
                            ServerMessage::IncorrectGuess(match_id, active_match.attempts),
                        )
                        .await?;
                    }
                    MatchState::Solved => {
                        send_message(
                            connections,
                            &active_match.challenger,
                            ServerMessage::MatchEnded(
                                match_id,
                                active_match.attempts,
                                active_match.hints.len() as u32,
                                true,
                            ),
                        )
                        .await?;
                        send_message(
                            connections,
                            &active_match.guesser,
                            ServerMessage::MatchEnded(
                                match_id,
                                active_match.attempts,
                                active_match.hints.len() as u32,
                                true,
                            ),
                        )
                        .await?;
                        server_state.finish_match(match_id);
                    }
                    // No actions needed
                    MatchState::GivenUp => {
                        server_state.finish_match(match_id);
                    }
                    MatchState::Cancelled => {
                        server_state.finish_match(match_id);
                    }
                }
            } else {
                send_message(
                    connections,
                    player_id,
                    ServerMessage::BadRequest(ClientRequestError::Match404),
                )
                .await?;
            }
        }
        ClientMessage::SendHint(match_id, hint) => {
            if let Some(active_match) = server_state.active_matches.get_mut(&match_id) {
                active_match.add_hint(&hint);
                send_message(
                    connections,
                    &active_match.guesser,
                    ServerMessage::MatchHint(match_id, hint),
                )
                .await?;
            } else {
                send_message(
                    connections,
                    player_id,
                    ServerMessage::BadRequest(ClientRequestError::Match404),
                )
                .await?;
            }
        }
        ClientMessage::GiveUp(match_id) => {
            if let Some(active_match) = server_state.active_matches.get_mut(&match_id) {
                if active_match.guesser.ne(player_id) {
                    send_message(
                        connections,
                        player_id,
                        ServerMessage::BadRequest(ClientRequestError::PermissionDenied),
                    )
                    .await?;
                    return Ok(());
                }
                active_match.give_up();
                send_message(
                    connections,
                    &active_match.guesser,
                    ServerMessage::MatchEnded(
                        match_id,
                        active_match.attempts,
                        active_match.hints.len() as u32,
                        false,
                    ),
                )
                .await?;
                server_state.finish_match(match_id);
            } else {
                send_message(
                    connections,
                    player_id,
                    ServerMessage::BadRequest(ClientRequestError::Match404),
                )
                .await?;
            }
        }
        ClientMessage::LeaveGame => {
            // Check if player was in a guesser in active games
            let mut matches_to_finish = Vec::<Uuid>::new();
            let guesser_matches = server_state
                .active_matches
                .values_mut()
                .filter(|active_match| active_match.guesser.eq(player_id));

            for active_match in guesser_matches {
                active_match.give_up();

                send_message(
                    connections,
                    &active_match.challenger,
                    ServerMessage::MatchEnded(
                        active_match.id,
                        active_match.attempts,
                        active_match.hints.len() as u32,
                        false,
                    ),
                )
                .await?;
                matches_to_finish.push(active_match.id);
            }

            let challenger_matches = server_state
                .active_matches
                .values_mut()
                .filter(|active_match| active_match.challenger.eq(player_id));

            for active_match in challenger_matches {
                active_match.cancel();

                send_message(
                    connections,
                    &active_match.guesser,
                    ServerMessage::MatchEnded(
                        active_match.id,
                        active_match.attempts,
                        active_match.hints.len() as u32,
                        false,
                    ),
                )
                .await?;
                matches_to_finish.push(active_match.id);
            }

            matches_to_finish.iter().for_each(|match_id| {
                server_state.finish_match(*match_id);
            });

            server_state.remove_available_player(player_id);
        }
    }

    Ok(())
}

async fn drop_all_connections(
    active_connections: &mut ActiveConnections,
) -> Result<(), anyhow::Error> {
    for connection in active_connections.write().await.values_mut() {
        connection.tx.send(ServerMessage::Disconnect).await?;
    }
    Ok(())
}

// TODO Documentation
// TODO readme documentation
