use tokio::sync::mpsc::Sender;

use crate::protocol::ServerMessage;

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
