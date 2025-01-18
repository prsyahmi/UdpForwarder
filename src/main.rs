use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc};
use tokio::sync::{Mutex, RwLock};
use tokio::net::UdpSocket;
use tokio::io::Result;

struct ClientMap {
    initialized: bool,
    client_addr: SocketAddr,
    client_sock: Arc<UdpSocket>,
    server_sock: UdpSocket,
    last_activity: u32,
    buffer: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let socket1 = UdpSocket::bind("127.0.0.1:27088").await?;
    let sock_arc = Arc::new(socket1);
    let sock_arc2 = Arc::clone(&sock_arc);
    let clients: HashMap<SocketAddr, ClientMap> = HashMap::new();
    let clients_arc = Arc::new(RwLock::new(clients));

    let clients_clone = Arc::clone(&clients_arc);
    let clients_clone2 = Arc::clone(&clients_arc);

    let j = tokio::spawn(async move {
        loop {
            let _ = handle_from_client(&sock_arc, &clients_clone).await;
        }
    });

    tokio::spawn(async move {
        let _: Result<()> = async move {
            let mut i = 0;
            loop {
                for (k, v) in clients_clone2.read().await.iter() {
                    let mut buf = vec![0u8; 2048];
                    let (len, addr) = v.server_sock.recv_from(&mut buf).await?;
                    println!("Received {} bytes from server: {}", len, addr);
                    let r = v.client_sock.send_to(&buf[..len], v.client_addr).await?;
                    println!("Sent {} bytes to client", r);
                }
            }

            Ok(())
        }.await;
    });

    let _ = tokio::join!(j);

    Ok(())
}

async fn create_client(addr: SocketAddr, socket: &Arc<UdpSocket>) -> ClientMap {
    println!("Create client {}", addr);

    let cm = ClientMap {
        initialized: false,
        buffer: vec![0u8; 2048],
        client_addr: addr,
        last_activity: 0,
        server_sock: UdpSocket::bind("0.0.0.0:0").await.unwrap(),
        client_sock: Arc::clone(socket),
    };

    return cm;
}

async fn handle_from_client(socket: &Arc<UdpSocket>, clients: &Arc<RwLock<HashMap<SocketAddr, ClientMap>>>) -> Result<()> {
    println!("Handle from client!");

    let mut buf = vec![0u8; 2048];
    let (len, addr) = socket.recv_from(&mut buf).await?;
    println!("Received {} bytes from client: {}", len, addr);

    let csread = clients.read().await;
    let csm = csread.get(&addr);
    let cs: &ClientMap;

    match csm {
        Some(v) => {
            cs = v;
        }
        None => {
            drop(csread);

            let new_map = create_client(addr, socket).await;
            { clients.write().await.insert(addr, new_map); }

            let csread = clients.read().await;
            let csm = csread.get(&addr).unwrap().clone();
            cs = csm;
        }
    }

    let server_addr = SocketAddr::from(([10, 11, 12, 1], 27015));
    println!("Sending {} bytes to server", len);
    let r = cs.server_sock.send_to(&buf[..len], server_addr).await?;
    println!("Sent {} bytes to server", r);

    Ok(())
}

/*
    let socket_clone = Arc::clone(socket);

    tokio::spawn(async move {
        let _: Result<()> = async move {
            let mut i = 0;
            loop {
                i += 1;
                println!("Handle from server! {}", i);

                let mut buf = vec![0u8; 2048];
                let (len, addr) = cm.server_sock.recv_from(&mut buf).await?;
                println!("Received {} bytes from server: {}", len, addr);
                let r = socket_clone.send_to(&buf[..len], cm.client_addr).await?;
                println!("Sent {} bytes to client", r);
            }

            Ok(())
        }.await;
    });
*/