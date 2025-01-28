use log::info;
use std::path::Path;
use tokio::{
    net::{TcpStream, UnixStream},
    sync::mpsc::Sender,
};

use crate::{
    connection::handle_stream,
    protocol::{ClientMessage, ServerMessage},
};

pub enum ClientConnection {
    Tcp(TcpStream),
    Unix(UnixStream),
}

pub async fn handle_server_connection(
    connection: ClientConnection,
    output_tx: Sender<ServerMessage>,
) -> Result<Sender<ClientMessage>, anyhow::Error> {
    match connection {
        ClientConnection::Tcp(stream) => handle_stream(stream, output_tx).await,
        ClientConnection::Unix(stream) => handle_stream(stream, output_tx).await,
    }
}

pub async fn create_connection(input: &str) -> Result<ClientConnection, anyhow::Error> {
    if is_valid_sock_path(input) {
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
