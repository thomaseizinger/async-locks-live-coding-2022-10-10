use async_std::net::{Incoming, TcpListener, TcpStream};
use asynchronous_codec::{Framed, LinesCodec};
use futures::{future, SinkExt, StreamExt};
use std::task::{Context, Poll};
use std::{io, mem};

#[async_std::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();

    let mut ping_counter = PingCounter::new(listener.incoming());

    loop {
        future::poll_fn(|cx| ping_counter.poll(cx)).await.unwrap();
    }
}

struct PingCounter<'a> {
    incoming: Incoming<'a>,
    num_pings: u64,
    streams: Vec<SocketState>,
}

enum SocketState {
    ReadyToReceive(Framed<TcpStream, LinesCodec>),
    Sending {
        msg: String,
        socket: Framed<TcpStream, LinesCodec>,
    },
    Flushing(Framed<TcpStream, LinesCodec>),
    Closed,
    Poisoned,
}

impl SocketState {
    fn poll(&mut self, num_pings: &mut u64, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            match mem::replace(self, SocketState::Poisoned) {
                SocketState::ReadyToReceive(mut socket) => match socket.poll_next_unpin(cx)? {
                    Poll::Ready(Some(line)) => match line.as_str() {
                        "ping\n" => {
                            *num_pings += 1;
                            *self = SocketState::Sending {
                                msg: format!("{num_pings}\n"),
                                socket,
                            };
                        }
                        _ => {
                            *self = SocketState::ReadyToReceive(socket);
                        }
                    },
                    Poll::Ready(None) => {
                        *self = SocketState::Closed;
                    }
                    Poll::Pending => {
                        *self = SocketState::ReadyToReceive(socket);
                        return Poll::Pending;
                    }
                },
                SocketState::Sending { msg, mut socket } => match socket.poll_ready_unpin(cx)? {
                    Poll::Ready(()) => {
                        socket.start_send_unpin(msg)?;
                        *self = SocketState::Flushing(socket);
                    }
                    Poll::Pending => {
                        *self = SocketState::Sending { msg, socket };
                        return Poll::Pending;
                    }
                },
                SocketState::Flushing(mut socket) => match socket.poll_flush_unpin(cx)? {
                    Poll::Ready(()) => {
                        *self = SocketState::ReadyToReceive(socket);
                    }
                    Poll::Pending => {
                        *self = SocketState::Flushing(socket);
                        return Poll::Pending;
                    }
                },
                SocketState::Poisoned => {
                    panic!("poisoned")
                }
                SocketState::Closed => {
                    return Poll::Pending;
                }
            }
        }
    }
}

impl<'a> PingCounter<'a> {
    fn new(incoming: Incoming<'a>) -> Self {
        Self {
            incoming,
            num_pings: 0,
            streams: Default::default(),
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            for stream in &mut self.streams {
                let _ = stream.poll(&mut self.num_pings, cx)?;
            }

            if let Poll::Ready(Some(stream)) = self.incoming.poll_next_unpin(cx)? {
                self.streams
                    .push(SocketState::ReadyToReceive(Framed::new(stream, LinesCodec)));
                continue;
            }

            return Poll::Pending;
        }
    }
}
