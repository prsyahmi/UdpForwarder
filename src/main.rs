use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};
use tokio::io::Result;

const SERVER: Token = Token(0);
const BUFFER_SIZE: usize = 2048;
const KIND_SERVER: u32 = 0;
const KIND_CLIENT: u32 = 1;

struct SocketKind {
    kind: u32,
    target_addr: SocketAddr,
    src: Arc<UdpSocket>,
    target: Option<Arc<UdpSocket>>,
}

fn main() -> Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(256);

    let mut clients: HashMap<String, UdpSocket> = HashMap::new();
    let mut token_to_clients: HashMap<usize, String> = HashMap::new();
    let mut token_to_socket: RefCell<HashMap<usize, SocketKind>> = RefCell::new(HashMap::new());

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
            src: Arc::new(socket),
            target: None,
            kind: KIND_SERVER,
            target_addr,
        };
        token_to_socket.borrow_mut().insert(token, m);

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

                    let mut token_sock = token_to_socket.borrow_mut();
                    let sock = match token_sock.get_mut(&token_id) {
                        Some(v) => v,
                        None => continue,
                    };

                    //if sock.kind == KIND_SERVER {
                        while let Ok((len, client_addr)) = sock.src.recv_from(&mut buf) {
                            let target = match &sock.target {
                                Some(v) => Arc::clone(v),
                                None => {
                                    let mut client_socket = UdpSocket::bind("0.0.0.0:0".parse().unwrap()).expect("Could not bind client socket");
        
                                    let token = get_token();
                                    poll.registry().register(&mut client_socket, Token(token), Interest::READABLE)?;

                                    let client_arc = Arc::new(client_socket);
                                    sock.target = Some(Arc::clone(&client_arc));

                                    let m = SocketKind {
                                        src: Arc::clone(&client_arc),
                                        //target: Some(Arc::clone(&sock.src)),
                                        target: None,
                                        kind: KIND_CLIENT,
                                        target_addr: "0.0.0.0:0".parse().unwrap(),
                                    };

                                    token_sock.insert(token, m);
                                    Arc::clone(&client_arc)
                                }
                            };

                            // New UDP socket (client) -> Real server
                            target.send_to(&buf[..len], sock.target_addr).expect("Could not send data to real server");
                        }
                    /*} else {
                        while let Ok((len, client_addr)) = sock.src.recv_from(&mut buf) {
                        }
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
                    }*/
                }
            }
        }
    }

}
