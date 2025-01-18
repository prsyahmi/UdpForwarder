use std::collections::HashMap;
use std::net::SocketAddr;
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};
use tokio::io::Result;

const SERVER: Token = Token(0);
const BUFFER_SIZE: usize = 2048;

fn main() -> Result<()> {
    let target_addr: SocketAddr = "10.11.12.1:27015".parse().unwrap();
    let mut socket1 = UdpSocket::bind("127.0.0.1:27088".parse().unwrap())?;
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(256);
    
    poll.registry().register(&mut socket1, SERVER, Interest::READABLE)?;

    let mut clients: HashMap<String, UdpSocket> = HashMap::new();
    let mut token_to_clients: HashMap<usize, String> = HashMap::new();
    let mut buf = vec![0u8; BUFFER_SIZE];
    let mut next_token: usize = 1;

    loop {
        poll.poll(&mut events, None)?;

        for event in &events {
            match event.token() {
                SERVER => {
                    if !event.is_readable() {
                        continue;
                    }
                    
                    while let Ok((len, addr)) = socket1.recv_from(&mut buf) {
                        let client_addr = addr.to_string();

                        let client_socket = clients.entry(client_addr.clone()).or_insert_with(|| {
                            let mut client_socket = UdpSocket::bind("0.0.0.0:0".parse().unwrap()).expect("Could not bind client socket");

                            let client_idx = next_token;
                            next_token += 1;

                            token_to_clients.insert(client_idx, client_addr);
                            poll.registry().register(&mut client_socket, Token(client_idx), Interest::READABLE).expect("Could not register client socket");

                            client_socket
                        });
                        //println!("Recv {} bytes from client {} -> server {}", len, addr, target_addr);
                        client_socket.send_to(&buf[..len], target_addr).expect("Could not send data to client");
                    }
                }
                Token(client_id) => {
                    if !event.is_readable() {
                        continue;
                    }

                    if let Some(client_addr) = token_to_clients.get(&client_id) {
                        if let Some(client_socket) = clients.get(client_addr) {
                            loop {
                                let read_res = client_socket.recv_from(&mut buf);
                                if read_res.is_err() {
                                    break;
                                }

                                let (len, _) = read_res.unwrap();
                                //println!("Recv {} bytes from server {} -> client {}", len, addr, client_addr);
                                let _ = socket1.send_to(&buf[..len], client_addr.parse().unwrap());
                            }
                        }
                    }
                }
            }
        }
    }

}
