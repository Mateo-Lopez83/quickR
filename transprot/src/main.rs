use std::env;
use transprot::{stuntry, udp};
use tokio::net::UdpSocket;
use std::io::{self, Write};
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::time::{sleep, Duration};

//main.rs
#[tokio::main]
async fn main() {
    let devtype:String = env::args().nth(1).expect("you have to specify if this is a sender or receiver");
    let conn = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
    let stun_server = "stun4.l.google.com:19302";
    let public_ip = stuntry::discover_public_address(stun_server, conn.clone()).await.unwrap();
    println!("The public address is: {}:{}", &public_ip.ip, &public_ip.port);
    match devtype.as_str() {
        "send"=>{
            //let address = env::args().nth(2).expect("you have to specify the ip address of the receiver");
            print!("Enter the destination address as ip:port: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            let address = input.trim().to_string();
            let remote_addr: SocketAddr = address.parse().unwrap();
            //se actualiza la conexión desde el STUN server al destinatario real
            conn.connect(remote_addr).await.unwrap();
            udp::advanced_hole_punching(conn.clone()).await.unwrap();
            let _ = udp::main_sending_process(conn).await;
        },
        "receive"=>{            
            print!("Enter the destination address as ip:port: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            let address = input.trim().to_string();
            let remote_addr: SocketAddr = address.parse().unwrap();
            //se actualiza la conexión desde el STUN server al destinatario real
            conn.connect(remote_addr).await.unwrap();
            udp::advanced_hole_punching(conn.clone()).await.unwrap();
            let _ = udp::receiving_process(conn).await.unwrap();

        },
        // "stun"=>{
        //     let conn = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        //     let example_stun_server = "stun4.l.google.com:19302";
        //     let public_ip = stuntry::discover_public_address(example_stun_server, conn).await.unwrap();
            
        // }
        othe =>{
            println!("No command associated with {}",othe);
        }

        
    }
}
