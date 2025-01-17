use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::UdpSocket;
use tokio::io::Result;
use tokio::task;

struct ClientMap {
    client_addr: SocketAddr,
    server_sock: UdpSocket,
    last_activity: u32,
    buffer: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let socket1 = Arc::new(UdpSocket::bind("127.0.0.1:27088").await?);

    let clients_back: HashMap<SocketAddr, ClientMap> = HashMap::new();
    let clients: Arc<Mutex<HashMap<SocketAddr, ClientMap>>> = Arc::new(Mutex::new(clients_back));
    let clients_clone = Arc::clone(&clients);

    loop {
        let s = handle_from_client(&socket1, &clients_clone).await;
        if let Err(e) = s {
            println!("Error! {}", e);
        }

        let socket1_clone = Arc::clone(&socket1);
        let clients_clone = Arc::clone(&clients_clone);

        task::spawn(async move {
            let mut cs = clients_clone.lock().await;
            for (key, map) in cs.iter_mut() {
                let (len, _) = map.server_sock.recv_from(&mut map.buffer).await.unwrap();

                let _ = socket1_clone.send_to(&map.buffer[..len], map.client_addr).await;
            }
        });
    }
}

async fn create_client(addr: SocketAddr) -> ClientMap {
    ClientMap {
        buffer: vec![0u8; 2048],
        client_addr: addr,
        last_activity: 0,
        server_sock: UdpSocket::bind("0.0.0.0:0").await.unwrap(),
    }
}

async fn handle_from_client(socket: &Arc<UdpSocket>, clients: &Arc<Mutex<HashMap<SocketAddr, ClientMap>>>) -> Result<()> {
    //println!("Handle from client!");

    let mut buf = vec![0u8; 2048];
    let (len, addr) = socket.recv_from(&mut buf).await?;
    //println!("Received {} bytes from client: {}", len, addr);

    let mut cs = clients.lock().await;
    let client = cs.entry(addr).or_insert_with(|| {
        tokio::task::block_in_place(|| futures::executor::block_on(create_client(addr)))
    });

    let server_addr = SocketAddr::from(([10, 11, 12, 1], 27015));
    client.server_sock.send_to(&buf[..len], server_addr).await?;

    Ok(())
}
