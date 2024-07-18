use anyhow::Result;
use bytes::Bytes;
use nom::{bytes::complete::take, combinator::peek, Parser};
use parser::{message, Message};
use tokio::net::UdpSocket;
use zeromq::{PubSocket, Socket, SocketSend, ZmqMessage};

mod parser;

const BUF_SIZE: usize = 1024;

#[tokio::main]
async fn main() -> Result<()> {
    let mut pubsock = PubSocket::new();
    pubsock.bind("tcp://0.0.0.0:5006").await?;

    let sock = UdpSocket::bind("0.0.0.0:5005").await?;
    let mut buf = [0u8; BUF_SIZE];
    loop {
        let _ = sock.recv(&mut buf).await?;

        // To process the packet
        // 1. Dissect the packet into messages
        // 2. Messages are broadcasted to all clients, with RTIO channel as topic.
        //
        // ZeroMQ (also zmq.rs) does publisher side filtering so the network throughput is not a
        // concern
        //
        let mut input = &buf[..];

        // Peek the message and return the corresponding raw message as well
        while let Ok((i, (msg, msg_raw))) = peek(message()).and(take(32usize)).parse(input) {
            input = i;
            match msg {
                Message::Sample { channel, .. } => {
                    // TODO: The zeromq is not very satisfactory as it's quite allocation intensive

                    // The RTIO channel as channel topic
                    let mut m = ZmqMessage::from(channel.to_string().as_str());
                    m.push_front(Bytes::copy_from_slice(msg_raw));
                    pubsock.send(m).await?;
                }
                x => {
                    dbg!(x);
                }
            }
        }
        // Each packet should contain certain number of messsages exactly
        assert_eq!(input.len(), 0)
    }
}
