use std::env;
use transprot::{stuntry, udp};
use tokio::net::UdpSocket;



#[tokio::main]
async fn main() {
    let devtype:String = env::args().nth(1).expect("you have to specify if this is a sender or receiver");
    match devtype.as_str() {
        "send"=>{
            let address = env::args().nth(2).expect("you have to specify the ip address of the receiver");
            let _ = udp::main_sending_process(address).await;
        },
        "receive"=>{
            let conn = UdpSocket::bind("0.0.0.0:0").await.unwrap();
            let _ = udp::receiving_process(conn).await.unwrap();
        },
        "stun"=>{
            let conn = UdpSocket::bind("0.0.0.0:0").await.unwrap();
            let example_stun_server = "stun4.l.google.com:19302";
            let public_ip = stuntry::discover_public_address(example_stun_server, conn).await.unwrap();
            
        }
        othe =>{
            println!("No command associated with {}",othe);
        }

        
    }
}
