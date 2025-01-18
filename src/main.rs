use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};
use tokio::io::Result;

const SERVER: Token = Token(0);
const BUFFER_SIZE: usize = 2048;
const KIND_SERVER: u32 = 0;
const KIND_CLIENT: u32 = 1;

struct SocketKind {
    kind: u32,
    token: usize,
    target_addr: SocketAddr,
    src: Arc<UdpSocket>,
    target: Option<Arc<UdpSocket>>,
}

fn main() -> Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(256);

    let mut token_to_socket: HashMap<usize, SocketKind> = HashMap::new();

    let mut buf = vec![0u8; BUFFER_SIZE];
    let mut next_token: usize = 0;

    let mut get_token = || {
        next_token += 1;
        next_token
    };

    let mut add_server = |listen_addr: &str, target_addr: &str| -> Result<()> {
        let token = get_token();
        let target_addr: SocketAddr = target_addr.parse().unwrap();
        let mut socket = UdpSocket::bind(listen_addr.parse().unwrap())?;
        
        poll.registry().register(&mut socket, Token(token), Interest::READABLE)?;
        
        let m = SocketKind {
            kind: KIND_SERVER,
            token,
            src: Arc::new(socket),
            target: None,
            target_addr,
        };
        token_to_socket.insert(token, m);

        Ok(())
    };

    add_server("127.0.0.1:27088", "10.11.12.1:27015").expect("Unable to add");

    loop {
        poll.poll(&mut events, None)?;

        for event in &events {
            match event.token() {
                Token(token_id) => {
                    if !event.is_readable() {
                        continue;
                    }

                    let sock = match token_to_socket.get_mut(&token_id) {
                        Some(v) => v,
                        None => continue,
                    };

                    let mut to_be_insert: Vec<SocketKind> = Vec::new();

                    while let Ok((len, client_addr)) = sock.src.recv_from(&mut buf) {
                        println!("Got {} bytes packet from {}", len, client_addr);

                        let target = match &sock.target {
                            Some(v) => Arc::clone(v),
                            None => {
                                let mut client_socket = UdpSocket::bind("0.0.0.0:0".parse().unwrap()).expect("Could not bind client socket");
    
                                let token = get_token();
                                poll.registry().register(&mut client_socket, Token(token), Interest::READABLE)?;

                                let client_arc = Arc::new(client_socket);
                                sock.target = Some(Arc::clone(&client_arc));

                                let m = SocketKind {
                                    kind: KIND_CLIENT,
                                    token,
                                    src: Arc::clone(&client_arc),
                                    target: Some(Arc::clone(&sock.src)),
                                    target_addr: client_addr,
                                };

                                to_be_insert.push(m);

                                //token_to_socket.insert(token, m);
                                Arc::clone(&client_arc)
                            }
                        };

                        // New UDP socket (client) -> Real server
                        println!("Sending {} bytes packet to {}", len, sock.target_addr);
                        target.send_to(&buf[..len], sock.target_addr).expect("Could not send data to real server");
                    }

                    while let Some(v) = to_be_insert.pop() {
                        token_to_socket.insert(v.token, v);
                    }
                }
            }
        }
    }

}
