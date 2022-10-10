use async_std::net::TcpListener;
use asynchronous_codec::{Framed, LinesCodec};
use futures::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    num_pings: u64,
}

#[async_std::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
    let state = Arc::new(Mutex::new(State::default()));

    loop {
        let (stream, _) = listener.accept().await.unwrap();

        async_std::task::spawn({
            let state = state.clone();
            let mut framed = Framed::new(stream, LinesCodec);

            async move {
                while let Some(Ok(msg)) = framed.next().await {
                    match msg.as_str() {
                        "ping\n" => {
                            let num_pings = {
                                let mut guard = state.lock().unwrap();

                                guard.num_pings += 1;

                                guard.num_pings
                            };

                            framed.send(format!("{num_pings}\n")).await.unwrap();
                        }
                        _ => {}
                    }
                }
            }
        });
    }
}
