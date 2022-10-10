use async_std::io::stdin;
use async_std::net::{TcpListener, TcpStream};
use asynchronous_codec::{Framed, FramedRead};
use futures::future::Either;
use futures::{future, SinkExt, StreamExt};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct PingState {
    num_pings: u64,
}

#[async_std::main]
async fn main() {
    match std::env::args().nth(1) {
        None => {
            let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
            let state = Arc::new(Mutex::new(PingState::default()));

            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = Framed::new(stream, asynchronous_codec::LinesCodec);

                async_std::task::spawn({
                    let state = state.clone();

                    async move {
                        loop {
                            match stream.next().await.unwrap().unwrap().as_str() {
                                "ping\n" => {
                                    let pings = {
                                        let mut guard = state.lock().unwrap();
                                        guard.num_pings += 1;

                                        guard.num_pings
                                    };

                                    stream.send(format!("{}\n", pings)).await.unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                });
            }
        }
        Some(port) => {
            let stream = TcpStream::connect(format!("127.0.0.1:{port}"))
                .await
                .unwrap();

            let mut stream = Framed::new(stream, asynchronous_codec::LinesCodec);
            let mut stdin = FramedRead::new(stdin(), asynchronous_codec::LinesCodec);

            loop {
                match future::select(stream.next(), stdin.next()).await {
                    Either::Left((Some(Ok(stream_line)), _)) => {
                        let num_pings = stream_line.replace('\n', "").parse::<u64>().unwrap();

                        println!("Number of pings: {num_pings}")
                    }
                    Either::Right((Some(Ok(new_message)), _)) => {
                        stream.send(new_message).await.unwrap();
                    }
                    _ => return,
                }
            }
        }
    }
}
