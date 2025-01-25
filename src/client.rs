use anyhow::anyhow;
use client_state::{ClientState, State};
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
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, UnixStream},
    task::{self},
};
mod client_state;
mod connection;
mod protocol;

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
    let mut connection = create_connection(input).await?;

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

async fn send_message_to_server(connection: &mut ClientConnection, msg: ClientMessage) {
    trace!("Message about to be sent");
    let mut payload = Vec::new();
    msg.serialize(&mut Serializer::new(&mut payload)).unwrap();
    match connection {
        ClientConnection::Tcp(stream) => {
            stream.write_all(&payload).await;
        }
        ClientConnection::Unix(stream) => {
            stream.write_all(&payload).await;
        }
    }
    trace!("Message sent");
}

async fn wait_for_server_msg(
    connection: &mut ClientConnection,
) -> Result<ServerMessage, anyhow::Error> {
    let mut buf = vec![0; 1024];

    let bytes_received = match connection {
        ClientConnection::Tcp(stream) => {
            let mut buf_reader = BufReader::new(stream);
            buf_reader.read(&mut buf).await
        }
        ClientConnection::Unix(stream) => {
            let mut buf_reader = BufReader::new(stream);
            buf_reader.read(&mut buf).await
        }
    };

    match bytes_received {
        Ok(0) => {
            // Connection closed
            info!("Server disconnected");
            Ok(ServerMessage::Disconnect)
        }
        Ok(n) => {
            trace!("Message from server received: {:?}", &buf);
            trace!("Message from server received: {:?}", &buf);
            let msg = rmp_serde::from_slice::<ServerMessage>(&buf[..n])
                .map_err(|e| anyhow!("Error parsing ServerMessage: {e:?}"))?;
            Ok(msg)
        }
        Err(e) => Err(anyhow!("Error reading from server: {e:?}")),
    }
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

async fn create_connection(input: &str) -> Result<ClientConnection, anyhow::Error> {
    if is_valid_sock_path(input) {
        // If it's a Unix socket path
        info!("Attempting to connect to Unix socket: {}", input);
        let unix_stream = UnixStream::connect(input).await?;
        return Ok(ClientConnection::Unix(unix_stream));
    } else {
        // If it's a TCP URL (e.g., "127.0.0.1:8080")
        info!("Attempting to connect to TCP address: {}", input);
        let tcp_stream = TcpStream::connect(input).await?;
        return Ok(ClientConnection::Tcp(tcp_stream));
    }
}

fn is_valid_sock_path(path: &str) -> bool {
    let path = Path::new(path);
    path.exists() && path.extension().map_or(false, |ext| ext == "sock")
}

// TODO Respond for password

// TODO Documentation
// TODO readme documentation
