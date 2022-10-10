use async_std::net::{Incoming, TcpListener, TcpStream};
use asynchronous_codec::{Framed, LinesCodec};
use futures::{future, SinkExt, StreamExt};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::task::{Context, Poll};

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

    pending_messages: HashMap<SocketAddr, String>,

    streams: HashMap<SocketAddr, Framed<TcpStream, LinesCodec>>,
}

impl<'a> PingCounter<'a> {
    fn new(incoming: Incoming<'a>) -> Self {
        Self {
            incoming,
            num_pings: 0,
            pending_messages: Default::default(),
            streams: Default::default(),
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            for (addr, socket) in &mut self.streams {
                match self.pending_messages.entry(*addr) {
                    Entry::Occupied(entry) => {
                        if socket.poll_ready_unpin(cx).is_ready() {
                            socket.start_send_unpin(entry.remove())?;
                        }
                    }
                    Entry::Vacant(vacant) => {
                        if let Poll::Ready(Some(new_message)) = socket.poll_next_unpin(cx)? {
                            match new_message.as_str() {
                                "ping\n" => {
                                    self.num_pings += 1;

                                    vacant.insert(format!("{}\n", self.num_pings));
                                    continue;
                                }
                                _ => {}
                            }
                        }
                    }
                }

                let _ = socket.poll_flush_unpin(cx)?;
            }

            if let Poll::Ready(Some(stream)) = self.incoming.poll_next_unpin(cx)? {
                let addr = stream.peer_addr()?;

                self.streams.insert(addr, Framed::new(stream, LinesCodec));
                continue;
            }

            return Poll::Pending;
        }
    }
}
