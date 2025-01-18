use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};
use tokio::io::Result;

const BUFFER_SIZE: usize = 2048;

struct SocketKind {
    token: usize,
    target_addr: SocketAddr,
    src: Arc<UdpSocket>,
    targets: HashMap<SocketAddr, Arc<UdpSocket>>,
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
        println!("  Listening on {} and forward to {}", listen_addr, target_addr);

        let token = get_token();
        let target_addr: SocketAddr = target_addr.parse().unwrap();
        let mut socket = UdpSocket::bind(listen_addr.parse().unwrap())?;
        
        poll.registry().register(&mut socket, Token(token), Interest::READABLE)?;
        
        let m = SocketKind {
            token,
            src: Arc::new(socket),
            target_addr,
            targets: HashMap::new(),
        };
        token_to_socket.insert(token, m);

        Ok(())
    };

    println!("UDPForward - created by syazaz");
    add_server("103.179.44.152:27015", "160.191.77.150:27015").expect("Unable to add");
    add_server("103.179.44.152:27016", "160.191.77.150:27016").expect("Unable to add");
    add_server("103.179.44.152:27017", "160.191.77.150:27017").expect("Unable to add");
    add_server("103.179.44.152:27018", "160.191.77.150:27018").expect("Unable to add");
    add_server("103.179.44.152:27019", "160.191.77.150:27019").expect("Unable to add");
    
    add_server("10.11.12.1:27015", "160.191.77.150:27015").expect("Unable to add");
    add_server("10.11.12.1:27016", "160.191.77.150:27016").expect("Unable to add");
    add_server("10.11.12.1:27017", "160.191.77.150:27017").expect("Unable to add");
    add_server("10.11.12.1:27018", "160.191.77.150:27018").expect("Unable to add");
    add_server("10.11.12.1:27019", "160.191.77.150:27019").expect("Unable to add");

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
                        //#[cfg(debug_assertions)]
                        //println!("Got {} bytes packet from {}", len, client_addr);

                        let target = match sock.targets.get(&client_addr) {
                            Some(v) => Arc::clone(v),
                            None => {
                                let mut client_socket = UdpSocket::bind("160.191.77.150:0".parse().unwrap()).expect("Could not bind client socket");
    
                                let token = get_token();
                                poll.registry().register(&mut client_socket, Token(token), Interest::READABLE)?;

                                let client_arc = Arc::new(client_socket);
                                sock.targets.insert(client_addr, Arc::clone(&client_arc));

                                let mut m = SocketKind {
                                    token,
                                    src: Arc::clone(&client_arc),
                                    target_addr: client_addr,
                                    targets: HashMap::new(),
                                };
                                m.targets.insert(sock.target_addr, Arc::clone(&sock.src));

                                to_be_insert.push(m);

                                //token_to_socket.insert(token, m);
                                Arc::clone(&client_arc)
                            }
                        };

                        // New UDP socket (client) -> Real server
                        #[cfg(debug_assertions)]
                        println!("{} -> {} || {} bytes", client_addr, sock.target_addr, len);
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
