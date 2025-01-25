use anyhow::anyhow;
use client_state::{ClientState, State};
use connection::{send_to_connection, Connection};
use indoc::printdoc;
use log::{debug, info, trace};
use protocol::{ClientMessage, ServerMessage};
use rmp_serde::Serializer;
use serde::Serialize;
use std::{
    env,
    io::{self, BufRead},
    path::Path,
    process,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpStream, UnixStream},
    task::{self},
};
mod client_state;
mod connection;
mod protocol;

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
    // TODO create connection depending on the path

    let input = &args[1];
    let mut connection: Connection;
    // let mut client_state = Arc::new(Mutex::new(ClientState::default()));
    let mut client_state = ClientState::default();

    if is_valid_sock_path(input) {
        info!("Attempting to connect to Unix socket: {}", input);
        connection = create_unix_connection(input).await?;
    } else if is_valid_tcp_url(input) {
        info!("Attempting to connect to TCP address: {}", input);
        // handle_tcp_connection(input);
        connection = create_tcp_connection(input).await?;
    } else {
        eprintln!("Invalid argument: {}", input);
        eprintln!("Expected a valid TCP URL or .sock file path.");
        process::exit(1);
    }

    info!("Connection successful");
    let mut _terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("Failed to register SIGTERM handler");

    // Client loop
    loop {
        debug!("Starting new loop: {:?}", &client_state);

        match &client_state.status {
            State::Initial | State::WaitingForPasswordValidation => {
                let server_msg = wait_for_server_msg(&mut connection).await?;
                client_state.update_from_server(server_msg);
            }
            State::WaitingForPassword => {
                printdoc!(
                    r#"
                        Welcome to WordGuesser.
                        Please authenticate yourself with a _not really secret_ **password**.
                    "#
                );
                let input = wait_for_user_input().await;
                client_state.update_from_user(&input);
            }
            State::SendPassword(password) => {
                printdoc! {
                    "Attempting to authenticate with provided password"
                };
                send_message_to_server(
                    &mut connection,
                    ClientMessage::AnswerPassword(password.to_string()),
                )
                .await;
                client_state.set_state(State::WaitingForPasswordValidation);
            }
            State::Quit => {
                printdoc!(
                    r#"
                        See you next time!
                    "#
                );
                break;
            } // _ => {}
        }
    }

    info!("Gracefully shutting down luxonis game client");
    Ok(())
}

async fn send_message_to_server(connection: &mut Connection, msg: ClientMessage) {
    trace!("Message about to be sent");
    let mut payload = Vec::new();
    msg.serialize(&mut Serializer::new(&mut payload)).unwrap();
    send_to_connection(connection, &payload).await;
    trace!("Message sent");
}

async fn create_unix_connection(path: &str) -> Result<Connection, anyhow::Error> {
    let stream = UnixStream::connect(path).await?;
    Ok(Connection::Unix(stream))
}

async fn create_tcp_connection(path: &str) -> Result<Connection, anyhow::Error> {
    let stream = TcpStream::connect(path).await?;
    Ok(Connection::Tcp(stream))
}

async fn wait_for_server_msg(connection: &mut Connection) -> Result<ServerMessage, anyhow::Error> {
    let mut buf = Vec::<u8>::new();
    match connection {
        Connection::Unix(ref mut stream) => {
            let mut buf_reader = BufReader::new(stream);
            buf_reader.read_until(b'\0', &mut buf).await?;
        }
        Connection::Tcp(ref mut stream) => {
            let mut buf_reader = BufReader::new(stream);
            buf_reader.read_until(b'\0', &mut buf).await?;
        }
    }
    trace!("Message from server received: {:?}", &buf);
    rmp_serde::from_slice::<ServerMessage>(&buf)
        .map_err(|e| anyhow!("Error parsing ServerMessage: {e:?}"))
}

async fn wait_for_user_input() -> String {
    let input = task::block_in_place(|| {
        let mut input = String::new();
        // Blocking call to read from stdin
        let stdin = io::stdin();
        stdin
            .lock()
            .read_line(&mut input)
            .expect("Failed to read line");
        input.trim().to_string() // Trim newline characters
    });
    input
}

fn is_valid_sock_path(path: &str) -> bool {
    let path = Path::new(path);
    path.exists() && path.extension().map_or(false, |ext| ext == "sock")
}

// TODO remove if not needed
fn is_valid_tcp_url(url: &str) -> bool {
    // url.starts_with("tcp://")
    true
}

// TODO Respond for password

// TODO Documentation
// TODO readme documentation
