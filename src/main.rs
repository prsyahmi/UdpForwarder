use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::UdpSocket;
use tokio::io::Result;

struct ClientMap {
    client_addr: SocketAddr,
    server_sock: UdpSocket,
    last_activity: u32,
    buffer: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let socket1 = UdpSocket::bind("127.0.0.1:27088").await?;
    let sock_arc = Arc::new(socket1);
    let clients: HashMap<SocketAddr, ClientMap> = HashMap::new();
    let clients_arc = Arc::new(Mutex::new(clients));

    let clients_clone = Arc::clone(&clients_arc);

    let j = tokio::spawn(async move {
        loop {
            let _ = handle_from_client(&sock_arc, &clients_clone).await;
        }
    });

    let _ = tokio::join!(j);

    Ok(())
}

async fn create_client(addr: SocketAddr, socket: &Arc<UdpSocket>, clients: &Arc<Mutex<HashMap<SocketAddr, ClientMap>>>) -> ClientMap {
    println!("Create client {}", addr);

    let cm = ClientMap {
        buffer: vec![0u8; 2048],
        client_addr: addr,
        last_activity: 0,
        server_sock: UdpSocket::bind("0.0.0.0:0").await.unwrap(),
    };

    let socket_clone = Arc::clone(socket);
    let clients_clone = Arc::clone(clients);

    tokio::spawn(async move {
        let _: Result<()> = async move {
            let mut i = 0;
            loop {
                i += 1;
                println!("Handle from server! {}", i);

                let cs: tokio::sync::MutexGuard<'_, HashMap<SocketAddr, ClientMap>> = clients_clone.lock().await;
                let client = cs.get(&addr).unwrap();
                let mut buf = vec![0u8; 2048];
                let (len, addr) = client.server_sock.recv_from(&mut buf).await?;
                println!("Received {} bytes from server: {}", len, addr);
                let r = socket_clone.send_to(&buf[..len], client.client_addr).await?;
                println!("Sent {} bytes to client", r);
            }

            Ok(())
        }.await;
    });

    return cm;
}

async fn handle_from_client(socket: &Arc<UdpSocket>, clients: &Arc<Mutex<HashMap<SocketAddr, ClientMap>>>) -> Result<()> {
    println!("Handle from client!");

    let mut buf = vec![0u8; 2048];
    let (len, addr) = socket.recv_from(&mut buf).await?;
    println!("Received {} bytes from client: {}", len, addr);

    let socket_clone = Arc::clone(socket);
    let clients_clone = Arc::clone(clients);

    tokio::spawn(async move {
        let mut cs = clients_clone.lock().await;
        println!("locked");
        let client = cs.entry(addr).or_insert_with(|| {
            tokio::task::block_in_place(|| futures::executor::block_on(create_client(addr, &socket_clone, &clients_clone)))
        });

        let server_addr = SocketAddr::from(([10, 11, 12, 1], 27015));
        println!("Sending {} bytes to server", len);
        let r = client.server_sock.send_to(&buf[..len], server_addr).await;
        //println!("Sent {} bytes to server", r);

    });

    Ok(())
}
