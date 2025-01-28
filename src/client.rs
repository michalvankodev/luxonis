use client_connection::{create_connection, handle_server_connection};
use client_state::{ClientState, State};
use log::{debug, error, info};
use std::{env, process};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader, Stdin},
    select, signal,
    sync::mpsc::{self},
};

mod client_connection;
mod client_state;
mod connection;
mod protocol;
mod validation;

/// Client application for "guess a word" game
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
                debug!("process {msg:?}");
                server_tx.send(msg).await?;
            }
        }

        if matches!(client_state.status, State::Quit) {
            break;
        }
    }

    info!("Gracefully shutting down luxonis game client");
    Ok(())
}

fn get_user_input_stream() -> tokio::io::Lines<BufReader<Stdin>> {
    let stdin = stdin();
    let reader = BufReader::new(stdin);
    reader.lines()
}
