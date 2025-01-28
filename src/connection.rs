use crate::protocol::ServerMessage;
use anyhow::anyhow;
use log::trace;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc::{self, Sender},
};

#[derive(Clone)]
pub struct Connection {
    pub tx: Sender<ServerMessage>,
}

// There is a possility to save ReadHalf and WriteHalf and have it be the same kind of type
// pub async fn send_to_connection<T>(stream: &mut WriteHalf<T>, payload: &[u8])
// where
//     T: AsyncWriteExt,
// {
//     let _ = stream.write_all(payload).await;
//     let _ = stream.write_all(b"\0").await;
// }

pub async fn handle_stream<S, OutgoingMessageType, IncommingMessageType>(
    stream: S,
    output_tx: Sender<IncommingMessageType>,
    // connections: &mut ActiveConnections,
) -> Result<Sender<OutgoingMessageType>, anyhow::Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    OutgoingMessageType: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug + Send + 'static,
    IncommingMessageType: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);

    // Create a channel for sending messages to this client
    let (client_tx, mut client_rx) = mpsc::channel::<OutgoingMessageType>(100);

    let _read_task = tokio::spawn({
        async move {
            // TODO optimize buffer
            let mut buf = vec![0; 1024];
            loop {
                trace!("at the start of the read task loop",);
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        // Connection closed
                        // println!("Client disconnected: {}", player_id);
                        break;
                    }
                    Ok(n) => {
                        // Process the message (e.g., routing or broadcasting)
                        trace!("Message from client received: {:?}", &buf);
                        if let Ok(msg) = rmp_serde::from_slice::<IncommingMessageType>(&buf[..n])
                            .map_err(|e| anyhow!("Error parsing {e:?}"))
                        {
                            trace!("Parsed Message from stream: {:?}", msg);
                            let _ = output_tx.send(msg).await;

                            trace!("Message sent to the output tx");
                        };
                    }
                    Err(e) => {
                        eprintln!("Error reading from incomming message{:?}", e);
                        break;
                    }
                }
                trace!("at the end of the read loop");
            }
        }
    });

    let _write_task = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            trace!("Sending msg {:?}", msg);
            let mut payload = Vec::new();
            msg.serialize(&mut Serializer::new(&mut payload)).unwrap();

            if writer.write_all(&payload).await.is_err() {
                eprintln!("Error writing to stream");
                break;
            }
            trace!("Message sent {:?}", msg);
        }
    });

    Ok(client_tx)
}
