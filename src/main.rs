use std::collections::HashMap;
use std::io::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use mio::net::UdpSocket;
use mio::{Events, Interest, Poll, Token};
use std::fs;
use serde::Deserialize;

const BUFFER_SIZE: usize = 2048;

// TODO: There is no release, no activity timeout
struct SocketKind {
    token: usize,
    target_addr: SocketAddr,
    src: Arc<UdpSocket>,
    targets: HashMap<SocketAddr, Arc<UdpSocket>>,
}

#[derive(Deserialize)]
struct Config {
    debug: Option<bool>,
    forwards: HashMap<String, String>,
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

    let config = fs::read_to_string("config.toml").expect("Unable to open config.toml");
    let config: Config = toml::from_str(&config).expect("Unable to read config.toml");
    let debug = config.debug.unwrap_or_else(|| false);

    println!("UDPForward - created by Team-MMG");
    for (k, v) in config.forwards {
        add_server(&k, &v).expect("Unable to add");
    }

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
                                let iface_bind = SocketAddr::new(sock.target_addr.ip(), 0);
                                let mut client_socket = UdpSocket::bind(iface_bind)
                                    .or_else(|_| UdpSocket::bind("0.0.0.0:0".parse().unwrap()))
                                    .expect("Could not bind client socket");
    
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
                        if debug {
                            println!("{} -> {} || {} bytes", client_addr, sock.target_addr, len);
                        }
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
