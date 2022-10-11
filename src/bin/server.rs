use async_std::net::{Incoming, TcpListener, TcpStream};
use asynchronous_codec::{Framed, LinesCodec};
use futures::{SinkExt, StreamExt};
use std::io;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

#[async_std::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();

    let mut ping_counter = PingCounter {
        num_pings: 0,
        incoming: listener.incoming(),
        streams: vec![],
    };

    loop {
        futures::future::poll_fn(|cx| ping_counter.poll(cx))
            .await
            .unwrap();
    }
}

struct PingCounter<'a> {
    num_pings: u64,
    incoming: Incoming<'a>,
    streams: Vec<SocketState>,
}

impl<'a> PingCounter<'a> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            for stream in &mut self.streams {
                match stream.poll(&mut self.num_pings, cx) {
                    Poll::Ready(Ok(())) => {}
                    Poll::Ready(Err(e)) => {
                        eprintln!("Stream failed: {e}")
                    }
                    Poll::Pending => {}
                }
            }

            match self.incoming.poll_next_unpin(cx)? {
                Poll::Ready(Some(stream)) => {
                    self.streams
                        .push(SocketState::ReadyToReceive(Framed::new(stream, LinesCodec)));
                    continue;
                }
                Poll::Ready(None) => return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into())),
                Poll::Pending => {}
            }

            return Poll::Pending;
        }
    }
}

enum SocketState {
    ReadyToReceive(Framed<TcpStream, LinesCodec>),
    SendMessage {
        msg: String,
        socket: Framed<TcpStream, LinesCodec>,
    },
    Flushing(Framed<TcpStream, LinesCodec>),
    Poisoned,
}

impl SocketState {
    fn poll(&mut self, num_pings: &mut u64, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            match std::mem::replace(self, SocketState::Poisoned) {
                SocketState::ReadyToReceive(mut socket) => match socket.poll_next_unpin(cx)? {
                    Poll::Ready(Some(line)) => match line.as_str() {
                        "ping\n" => {
                            *num_pings += 1;

                            *self = SocketState::SendMessage {
                                msg: format!("{num_pings}\n"),
                                socket,
                            }
                        }
                        _ => {
                            *self = SocketState::ReadyToReceive(socket);
                            continue;
                        }
                    },
                    Poll::Ready(None) => {
                        return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
                    }
                    Poll::Pending => {
                        *self = SocketState::ReadyToReceive(socket);
                        return Poll::Pending;
                    }
                },
                SocketState::SendMessage { msg, mut socket } => {
                    match socket.poll_ready_unpin(cx)? {
                        Poll::Ready(()) => {
                            socket.start_send_unpin(msg)?;

                            *self = SocketState::Flushing(socket);
                            continue;
                        }
                        Poll::Pending => {
                            *self = SocketState::SendMessage { msg, socket };
                            return Poll::Pending;
                        }
                    }
                }
                SocketState::Flushing(mut socket) => match socket.poll_flush_unpin(cx)? {
                    Poll::Ready(()) => {
                        *self = SocketState::ReadyToReceive(socket);
                        continue;
                    }
                    Poll::Pending => {
                        *self = SocketState::Flushing(socket);
                        return Poll::Pending;
                    }
                },
                SocketState::Poisoned => {
                    unreachable!("poisoned")
                }
            }
        }
    }
}
