use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UnixStream},
};

pub enum Connection {
    Tcp(TcpStream),
    Unix(UnixStream),
}

// There is a possility to save ReadHalf and WriteHalf and have it be the same kind of type
pub async fn send_to_connection(connection: &mut Connection, payload: &[u8]) {
    match connection {
        Connection::Tcp(stream) => {
            let (rx, mut wx) = stream.split();
            let _ = wx.write_all(payload).await;
            let _ = stream.write_all(b"\0").await;
        }
        Connection::Unix(stream) => {
            let _ = stream.write_all(payload).await;
            let _ = stream.write_all(b"\0").await;
        }
    }
}
