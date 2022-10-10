use async_std::io::stdin;
use async_std::net::TcpStream;
use asynchronous_codec::{Framed, FramedRead, LinesCodec};
use futures::future::Either;
use futures::{future, SinkExt, StreamExt};

#[async_std::main]
async fn main() {
    let port = std::env::args().nth(1).unwrap();

    let stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();

    let mut stream = Framed::new(stream, LinesCodec);
    let mut stdin = FramedRead::new(stdin(), LinesCodec);

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
