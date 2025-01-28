use anyhow::anyhow;
use log::{debug, info, trace};
use rmp_serde::Serializer;
use serde::Serialize;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::{self, Sender},
};
use uuid::Uuid;

use crate::{
    connection::{handle_stream, Connection},
    protocol::{ClientMessage, ClientRequestError, ServerMessage},
    server_state::{MatchState, ServerState},
    ActiveConnections,
};

/// Handle new connection
/// Create a new channel for communication with client
/// Save the channel in `connections` `HashMap` for an ability push communicate messages to them when needed
pub async fn handle_client<S>(
    stream: S,
    main_tx: Sender<(Uuid, ClientMessage)>,
    connections: &mut ActiveConnections,
) -> Result<(), anyhow::Error>
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

/// Sends a message to specific player
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

/// Process messages from clients and update `server_state` accordingly
/// React to messages and let other players know if there is an update
pub async fn react_to_client_msg(
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
            debug!("player leaving a game");
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

            debug!("Player is going to be removed");
            server_state.remove_available_player(player_id);
            send_message(connections, player_id, ServerMessage::Disconnect).await?;
        }
    }

    Ok(())
}
