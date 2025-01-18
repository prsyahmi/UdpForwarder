use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::sync::{Mutex, RwLock};
use tokio::net::UdpSocket;
use tokio::io::Result;
use tokio::time::sleep;

struct ClientMap {
    our_to_real: UdpSocket,
    last_activity: u32,
}

struct DataRecvClient {
    from_addr: SocketAddr,
    buffer: Vec<u8>,
}
impl DataRecvClient {
    fn new(addr: SocketAddr, slice: &[u8]) -> DataRecvClient {
        let m = DataRecvClient{
            from_addr: addr,
            buffer: slice.to_vec(),
        };
        m
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<DataRecvClient>();

    let socket1 = UdpSocket::bind("127.0.0.1:27088").await?;
    let sock_arc = Arc::new(socket1);
    let sock_arc2 = Arc::clone(&sock_arc);
    
    let clients: HashMap<SocketAddr, ClientMap> = HashMap::new();
    let clients_arc = Arc::new(Mutex::new(clients));
    let clients_arc2 = Arc::clone(&clients_arc);

    let runtime = Builder::new_multi_thread()
        .worker_threads(8) // Set the number of worker threads
        .enable_all()
        .build()
        .unwrap();

    let j = runtime.spawn(async move {
        loop {
            let _ = handle_from_client(&sock_arc, &tx).await;
        }
    });

    runtime.spawn(async move {
        loop {
            let v = rx.recv().unwrap();
            println!("Received {} bytes from client {}", v.buffer.len(), v.from_addr);

            let cs = clients_arc.lock().await;
            match cs.get(&v.from_addr) {
                Some(m) => {
                    println!("Found existing map for {}", v.from_addr);
                    let real_addr = SocketAddr::from(([10,11,12,1], 2015));
                    let _ = m.our_to_real.send_to(&v.buffer, real_addr).await;
                },
                None => {
                    drop(cs);
                    let m = create_client().await;

                    println!("Creating new map table for {}", v.from_addr);

                    {
                        let mut cs = clients_arc.lock().await;
                        cs.insert(v.from_addr, m);
                        let m = cs.get(&v.from_addr).unwrap();
                    
                        let real_addr = SocketAddr::from(([10,11,12,1], 2015));
                        let _ = m.our_to_real.send_to(&v.buffer, real_addr).await;
                    }

                    /*{
                        let cs = clients_arc.write().await;
                        let m = cs.get(&v.from_addr).unwrap();
                    
                        let real_addr = SocketAddr::from(([10,11,12,1], 2015));
                        let _ = m.our_to_real.send_to(&v.buffer, real_addr).await;
                    }*/

                    println!("Created new map table for {}", v.from_addr);
                }
            }
        }
    });

    runtime.spawn(async move {
        loop {
            let mut buf = vec![0u8; 2048];
            let cs = clients_arc2.lock().await;
            println!("Empty? {}", cs.is_empty());
            for (k, v) in cs.iter() {
                println!("{}", k);
                //println!("{}", v.our_to_real.peer_addr().unwrap());
                let res = v.our_to_real.recv_from(&mut buf).await;
                println!("{}ddd", k);
                match res {
                    Ok(res) => {
                        println!("Sending to {}", k);
                        let _ = sock_arc2.send_to(&buf[..res.0], k);
                    },
                    Err(_) => {}
                }

                //let _ = sock_arc2.send_to(&buf[..len], k);
                //let _ = handle_from_server(&v.our_to_real, &sock_arc2, &k).await;
            }
        }
    });

    /*tokio::spawn(async move {
        loop {

        }

        let _: Result<()> = async move {
            let mut i = 0;
            loop {
                for (k, v) in clients_clone2.read().await.iter() {
                    let mut buf = vec![0u8; 2048];
                    let (len, addr) = v.our_to_real.recv_from(&mut buf).await?;
                    println!("Received {} bytes from server: {}", len, addr);
                    let r = v.client_to_our.send_to(&buf[..len], v.client_addr).await?;
                    println!("Sent {} bytes to client", r);
                }
            }

            Ok(())
        }.await;
    });

    tokio::spawn(async move {
        let _: Result<()> = async move {
            let mut i = 0;
            loop {
                for (k, v) in clients_clone2.read().await.iter() {
                    let mut buf = vec![0u8; 2048];
                    let (len, addr) = v.our_to_real.recv_from(&mut buf).await?;
                    println!("Received {} bytes from server: {}", len, addr);
                    let r = v.client_to_our.send_to(&buf[..len], v.client_addr).await?;
                    println!("Sent {} bytes to client", r);
                }
            }

            Ok(())
        }.await;
    });*/

    let _ = tokio::join!(j);

    Ok(())
}

async fn create_client() -> ClientMap {
    let cm = ClientMap {
        last_activity: 0,
        our_to_real: UdpSocket::bind("0.0.0.0:0").await.unwrap(),
    };

    return cm;
}

async fn handle_from_client(socket: &Arc<UdpSocket>, tx: &mpsc::Sender<DataRecvClient>) -> Result<()> {
    let mut buf = vec![0u8; 2048];
    let (len, addr) = socket.recv_from(&mut buf).await?;

    let _ = tx.send(DataRecvClient::new(addr, &buf[..len]));

    Ok(())
}

async fn handle_from_server(real_socket: &UdpSocket, client_socket: &UdpSocket, target_addr: &SocketAddr) -> Result<()> {
    println!("handle from server");

    let mut buf = vec![0u8; 2048];
    let (len, addr) = real_socket.recv_from(&mut buf).await?;

    let _ = client_socket.send_to(&buf[..len], target_addr);
    println!("handled from server");

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