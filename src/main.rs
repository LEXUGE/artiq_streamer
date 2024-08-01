use anyhow::Result;
use clap::{Parser, Subcommand};
use log::{debug, info, warn};
use nom::{bytes::complete::take, combinator::peek, Parser as NomParser};
use parser::{message, Message};
use tokio::net::UdpSocket;

mod parser;

const BUF_SIZE: usize = 1024;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Publish {
        #[arg(default_value = "tcp://0.0.0.0:5006")]
        publisher_addr: String,
        #[arg(default_value = "0.0.0.0:5005")]
        udp_addr: String,
    },
    Set {
        kasli_addr: String,
        start: u64,
        end: u64,
    },
}

async fn publisher(publisher_addr: &str, udp_addr: &str) -> Result<()> {
    let ctx = zmq::Context::new();

    let pubsock = ctx.socket(zmq::PUB).unwrap();
    pubsock.bind(publisher_addr)?;

    let sock = UdpSocket::bind(udp_addr).await?;

    info!("Started");

    let mut buf = [0u8; BUF_SIZE];
    let mut prev_timestamp = 0u64;
    loop {
        let len = sock.recv(&mut buf).await?;

        // To process the packet
        // 1. Dissect the packet into messages
        // 2. Messages are broadcasted to all clients, with RTIO channel as topic.
        //
        // ZeroMQ (also zmq.rs) does publisher side filtering so the network throughput is not a
        // concern
        //
        let mut input = &buf[..len];

        debug!("Received packet of len: {}", len);

        // Peek the message and return the corresponding raw message as well
        // TODO: Support all packet format
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
                        warn!(
                            "Message out of order! Prev {}, Current {}, Position {}",
                            prev_timestamp,
                            timestamp,
                            input.len()
                        );
                    } else {
                        debug!(
                            "Normal timestamp order. Prev {}, Current {}",
                            prev_timestamp, timestamp
                        );
                    }
                    prev_timestamp = timestamp;

                    // The RTIO channel as channel topic
                    pubsock.send_multipart([channel.to_string().as_bytes(), msg_raw], 0)?;
                }
                Message::Stop { .. } => {
                    pubsock.send_multipart([b"STOP_CHANNEL", msg_raw], 0)?;
                    // // IGNORE reset of the packet
                    // continue 'packet;
                }
            }
        }
        // Each packet should contain certain number of messsages exactly
        assert_eq!(input.len(), 0)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    colog::init();

    let cli = Cli::parse();

    match cli.commands {
        Commands::Set {
            kasli_addr,
            start,
            end,
        } => {
            let s = UdpSocket::bind("0.0.0.0:0").await?;
            s.connect(kasli_addr).await?;
            s.send(&[start.to_be_bytes(), end.to_be_bytes()].concat())
                .await?;
            info!("streaming window setting has been sent.");
            Ok(())
        }
        Commands::Publish {
            publisher_addr,
            udp_addr,
        } => publisher(&publisher_addr, &udp_addr).await,
    }
}
