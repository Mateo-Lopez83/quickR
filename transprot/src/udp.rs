use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};
use tokio::sync::mpsc;
use bytes::Bytes;
use crate::header::Header;

//udp.rs
pub const PAYLOADSIZE:usize = 1180;
pub const MAXDATAGRAMSIZE:usize = 1300;

pub mod dummy_gen{
    use crate::header::{FragmentType, Header, ChannelType};
    use crate::udp::PAYLOADSIZE;
    use crate::{PacketError, RngExt};
    use bytes::{BufMut, Bytes, BytesMut};
    
    
    fn generate_header()->Header{
        let mut rng = rand::rng(); 
        let fragment = FragmentType::try_from(rand::random_range(1..5)).expect("Weird shi fragemtn");
        let version  = rand::random_range(1..=2);
        let padding = rand::random_bool(0.5);
        let channel = ChannelType::try_from(rand::random_range(1..4)).expect("Weird shi channel");
        let frametype = rand::random_bool(0.5);
        let sequence: u16 = rng.random(); 
        let size: u16 = PAYLOADSIZE as u16; 
        let timestamp: u32 = rng.random(); 
        let ssrc: u32 = rng.random(); 
        let head_example = Header::new(fragment,version,
                                             padding, 
                                             channel, 
                                             frametype, 
                                             sequence, 
                                             size, 
                                             timestamp, 
                                             ssrc);

        head_example

    }

    fn generate_fake_payload ()-> Bytes{
        let mut rng = rand::rng(); 
        let mut  payload = BytesMut::new();
        payload.resize(PAYLOADSIZE, rng.random());
        let payload = payload.freeze();

        payload
    }

    pub fn generate_fake_datagram()->Bytes{
        let header = generate_header().serialize().freeze();
        let payload = generate_fake_payload();
        let mut combined = BytesMut::with_capacity(header.len() + payload.len());
        combined.put_slice(&header);
        combined.put_slice(&payload);

        combined.freeze()
    }
}

async fn channel_fake_data_creator(tx:Sender<Bytes>){
    let mut interval = time::interval(Duration::from_millis(1000));
    for _ in 0..10{
        interval.tick().await; 
        let fake_payload = dummy_gen::generate_fake_datagram();
        //println!("{}", String::from_utf8_lossy(&fake_payload));
        if tx.send(fake_payload).await.is_err() {
            println!("consumer dropped, stopping generator");
            break;
        }
    }
   
}

async fn channel_consumer(mut rx: mpsc::Receiver<Bytes>, socket: Arc<UdpSocket>){
    while let Some(payload) = rx.recv().await {
        if let Err(e) = socket.send(&payload).await {
            eprintln!("send failed: {e}");
            }
        else {
            println!("Packet sent successfully!")
        }
        }
    println!("channel closed, no more senders");
    
}

pub async fn main_sending_process(conn: Arc<UdpSocket>) -> Result<(), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel::<Bytes>(30);
    let socket = conn.clone();
    //socket.connect(remote_addr).await?;
    //se hacen los lets para que retorne al menos un None y cuando ya acaben ambos pasa al join!
    let gen_handle = tokio::spawn(channel_fake_data_creator(tx));
    let consumer_handle = tokio::spawn(channel_consumer(rx, socket));

    let _ = tokio::join!(gen_handle, consumer_handle);
    
    

    Ok(())
}

// pub async fn receiving_process(stunaddress:XorMappedAddress,conn: Arc<UdpSocket>)->Result<(), Box<dyn Error>>{
pub async fn receiving_process(conn: Arc<UdpSocket>)->Result<(), Box<dyn Error>>{    
    // let local_ip = stunaddress.ip;
    // let port = stunaddress.port;
    // let local_addr = format!("{}:{}", local_ip,port.to_string());  
    // println!("Stun's IP address is {}", local_addr);
    let socket = conn.clone();
    let mut buf = [0u8; 1200];
    loop {
        let len = socket.recv(&mut buf).await?;
        println!(
        "Received {} bytes from connection",len);
        let mut received = Bytes::copy_from_slice(&buf[..len]);
        match Header::unserialize(&mut received) {
            Ok(header)=>{
                //println!("Got header from {addr}: {:?}", header);
                println!("Payload length: {}", received.len());
                println!("Sequence number: {}", header.sequence_number )
            }
            Err(e)=>{
                eprintln!("Failed to parse header: {:?}", e);
            }
            
        }
    }
    
}


pub async fn hole_punching(conn: Arc<UdpSocket>) -> Result<(), Box<dyn Error>>{
    conn.send(&[1]).await?;
    Ok(())
    
}

pub async fn advanced_hole_punching(conn: Arc<UdpSocket>)-> Result<(), Box<dyn Error>>{
    println!("Trying to connect to peer... Please wait...");
    let mut interval = time::interval(Duration::from_millis(500));
    let mut buf = [0u8; MAXDATAGRAMSIZE];
    for i in 1..15{
        tokio::select! {
            // Send a punching packet every 500 ms
            _ = interval.tick() => {
                conn.send(b"punch").await?;
                println!("Sent packet number {i}");
            }

            // Check whether the peer sent something
            result = conn.recv(&mut buf) => {
                let len = result?;

                if &buf[..len] == b"punch" || len==1197 {
                    println!("Peer connected. Connection status: strong");
                    return Ok(());
                }

                println!("Received {} bytes", len);
            }
        }
    }
    

    Ok(())
}