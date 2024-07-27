use anyhow::Result;
use nom::{bytes::complete::take, combinator::peek, Parser};
use parser::{message, Message};
use tokio::net::UdpSocket;

mod parser;

const BUF_SIZE: usize = 1024;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = zmq::Context::new();

    let pubsock = ctx.socket(zmq::PUB).unwrap();
    pubsock.bind("tcp://0.0.0.0:5006")?;

    let sock = UdpSocket::bind("0.0.0.0:5005").await?;
    let mut buf = [0u8; BUF_SIZE];
    let mut prev_timestamp = 0u64;
    'packet: loop {
        let len = sock.recv(&mut buf).await?;

        // To process the packet
        // 1. Dissect the packet into messages
        // 2. Messages are broadcasted to all clients, with RTIO channel as topic.
        //
        // ZeroMQ (also zmq.rs) does publisher side filtering so the network throughput is not a
        // concern
        //
        let mut input = &buf[..len];

        println!("Another packet!");

        // Peek the message and return the corresponding raw message as well
        // TODO: ignore unknown packet format of length of 32 bytes
        while let Ok((i, msg)) = peek(message()).parse(input) {
            let msg_raw;
            (input, msg_raw) = take::<_, _, nom::error::Error<_>>(msg.len())
                .parse(i)
                .unwrap();
            match msg {
                Message::Sample {
                    channel, timestamp, ..
                } => {
                    if prev_timestamp > timestamp {
                        dbg!(timestamp, prev_timestamp, input.len());
                    } else {
                        println!("normal prev {}, current {}", prev_timestamp, timestamp);
                    }
                    prev_timestamp = timestamp;

                    // The RTIO channel as channel topic
                    pubsock.send_multipart([channel.to_string().as_bytes(), msg_raw], 0)?;
                }
                Message::Stop { .. } => {
                    dbg!(msg);
                    pubsock.send_multipart([b"STOP_CHANNEL", msg_raw], 0)?;
                    // IGNORE reset of the packet
                    continue 'packet;
                }
            }
        }
        // Each packet should contain certain number of messsages exactly
        assert_eq!(input.len(), 0)
    }
}
