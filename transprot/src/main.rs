use std::env;
use transprot::{stuntry, udp};
use tokio::net::UdpSocket;
use std::io::{self, Write};
use std::sync::Arc;
use std::net::{SocketAddr};
//use tokio::time::{sleep, Duration};
use std::time::Instant;
use commons::ip_converter::{turn_code_to_ip, turn_ip_to_code};

//main.rs
#[tokio::main]
async fn main() {
    let mut timer: Instant;
    let devtype:String = env::args().nth(1).expect("you have to specify if this is a sender or receiver");
    let conn = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
    let stun_server = "stun4.l.google.com:19302";
    let public_ip = stuntry::discover_public_address(stun_server, conn.clone()).await.unwrap();
    let ip_string= turn_ip_to_code(&public_ip.ip, &public_ip.port);
    println!("Your code is: {}",  &ip_string );
    match devtype.as_str() {
        "send"=>{
            //let address = env::args().nth(2).expect("you have to specify the ip address of the receiver");
            loop{
                print!("Enter the destination code: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");

                let inpu2 = input.trim().to_string();
                let address = turn_code_to_ip(&inpu2);
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
                print!("Enter the destination code: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");
                
                
                let inpu2 = input.trim().to_string();
                let address = turn_code_to_ip(&inpu2);
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
            //let _ = udp::receiving_process(conn, timer).await.unwrap();

        },
        othe =>{
            println!("No command associated with {}",othe);
        }        
    }
}
