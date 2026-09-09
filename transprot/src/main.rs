use std::env;
use rand::RngExt;
use transprot::{stuntry, udp};
use tokio::net::UdpSocket;
use std::io::{self, Write};
use std::sync::Arc;
use std::net::SocketAddr;
//use tokio::time::{sleep, Duration};
use std::time::Instant;

//main.rs
#[tokio::main]
async fn main() {
    let mut rng = rand::rng(); 
    //let video_ssrc: u32 = rng.random();
    //let audio_ssrc: u32 = rng.random();
    let mut timer: Instant;
    let devtype:String = env::args().nth(1).expect("you have to specify if this is a sender or receiver");
    let conn = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
    let stun_server = "stun4.l.google.com:19302";
    let public_ip = stuntry::discover_public_address(stun_server, conn.clone()).await.unwrap();
    println!("The public address is: {}:{}", &public_ip.ip, &public_ip.port);
    match devtype.as_str() {
        "send"=>{
            //let address = env::args().nth(2).expect("you have to specify the ip address of the receiver");
            loop{
                print!("Enter the destination address as ip:port: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");


                let address = input.trim().to_string();
                let remote_addr: SocketAddr = match address.parse() {
                    Ok(addr) => addr,
                    Err(e) => {
                        println!("'{address}' isn't a valid ip:port address ({e}). Try again.");
                        continue;
                    }
                };
                //se actualiza la conexión desde el STUN server al destinatario real
                conn.connect(remote_addr).await.unwrap();
                timer = Instant::now();
                match udp::advanced_hole_punching(conn.clone(),timer).await{
                    Ok(_) => {
                        println!("Initializing packet sending.");
                        break;
                    }
                    Err(_) =>{
                        println!("Trying to connect again.");
                        continue
                    }
                }
            }
            let _ = udp::main_sending_process(conn, timer).await;
        },
        "receive"=>{            
            
            loop{
                print!("Enter the destination address as ip:port: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");
                
                
                let address = input.trim().to_string();
                let remote_addr: SocketAddr = match address.parse() {
                    Ok(addr) => addr,
                    Err(e) => {
                        println!("'{address}' isn't a valid ip:port address ({e}). Try again.");
                        continue;
                    }
                };
                //se actualiza la conexión desde el STUN server al destinatario real
                conn.connect(remote_addr).await.unwrap();
                timer = Instant::now();
                match udp::advanced_hole_punching(conn.clone(),timer).await{
                    Ok(_) => {
                        println!("Initializing receiver.");
                        break;
                    }
                    Err(_) =>{
                        println!("Trying to connect again.");
                        continue
                    }
                }
            }
            let _ = udp::receiving_process(conn).await.unwrap();

        },
        othe =>{
            println!("No command associated with {}",othe);
        }        
    }
}
