//! Puente WebSocket → TCP (rol "websockify" mínimo): permite que el cliente
//! noVNC del navegador hable VNC con QEMU, que solo escucha TCP. El WebSocket
//! de axum corre sobre tokio-tungstenite; los errores se ignoran a propósito:
//! el cierre de cualquiera de los dos extremos cierra el otro.

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn bridge(ws: WebSocket, tcp_port: u16) {
    let Ok(tcp) = TcpStream::connect(("127.0.0.1", tcp_port)).await else {
        return;
    };
    let (mut ws_sink, mut ws_stream) = ws.split();
    let (mut tcp_r, mut tcp_w) = tokio::io::split(tcp);

    // navegador → QEMU
    let up = async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            if let Message::Binary(data) = msg {
                if tcp_w.write_all(&data).await.is_err() {
                    break;
                }
            }
        }
    };
    // QEMU → navegador
    let down = async move {
        let mut buf = vec![0u8; 16384];
        loop {
            match tcp_r.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let frame = Message::Binary(Bytes::copy_from_slice(&buf[..n]));
                    if ws_sink.send(frame).await.is_err() {
                        break;
                    }
                }
            }
        }
    };
    tokio::select! {
        _ = up => {},
        _ = down => {},
    }
}
