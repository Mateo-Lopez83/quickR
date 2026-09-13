use std::error::Error;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::Sender;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};
use tokio::sync::mpsc;
use bytes::Bytes;
use crate::PacketError;
use crate::header::{ChannelType, HEADERSIZE, Header};

//udp.rs
pub const PAYLOADSIZE:usize = 1180;
pub const MAXDATAGRAMSIZE:usize = 1300;

pub mod dummy_gen{
use std::time::Instant;

use crate::header::{FragmentType, Header, ChannelType};
    use crate::udp::PAYLOADSIZE;
    use crate::{RngExt};
    use bytes::{BufMut, Bytes, BytesMut};
    
    
    fn generate_test_header(timestamp: u32, seqnum: u16)->Header{
        let mut rng = rand::rng(); 
        let fragment = FragmentType::try_from(rand::random_range(1..5)).expect("Weird shi fragemtn");
        let version  = rand::random_range(1..=2);
        let padding = rand::random_bool(0.5);
        let channel = ChannelType::try_from(rand::random_range(1..4)).expect("Weird shi channel");
        let frametype = rand::random_bool(0.5);
        let sequence: u16 = seqnum; 
        let size: u16 = PAYLOADSIZE as u16; 
        //let timestamp: u32 = rng.random(); 
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
    fn generate_header(fragmentnum: u8, 
                        version: u8,
                        padding: bool,
                        channelnum: u8,
                        frametype: bool,
                        sequence: u16,
                        size: u16,
                        timestamp: u32,
                        ssrc: u32)->Header{
         
        let fragment = FragmentType::try_from(fragmentnum).expect("Weird shi fragemtn");   
        let channel = ChannelType::try_from(channelnum).expect("Weird shi channel");
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

    pub fn generate_fake_datagram(timestamp: u32, seqnum: u16)->Bytes{
        let header = generate_test_header(timestamp, seqnum).serialize().freeze();
        let payload = generate_fake_payload();
        let mut combined = BytesMut::with_capacity(header.len() + payload.len());
        combined.put_slice(&header);
        combined.put_slice(&payload);

        combined.freeze()
    }

    pub fn generate_control_datagram(timer: Instant)->Bytes{
        let time_now = timer.elapsed().as_millis() as u32;

        //por ahora se envía exclusivamente un header sin payload como simulación de SYN/ACK protocl
        generate_header(4, 2, false, 4,true, 0,18,time_now,1).serialize().freeze()
    }
}

async fn channel_fake_data_creator(tx:Sender<Bytes>, timer: Instant){
    let mut interval = time::interval(Duration::from_millis(1000));

    let mut seqnum: u16 = 11;
    for _ in 0..10{
        interval.tick().await; 
        let timestamp = timer.elapsed().as_millis() as u32;
        let fake_payload = dummy_gen::generate_fake_datagram(timestamp, seqnum);
        seqnum = seqnum +1;
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

pub async fn main_sending_process(conn: Arc<UdpSocket>, timer: Instant) -> Result<(), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel::<Bytes>(30);
    let socket = conn.clone();
    //socket.connect(remote_addr).await?;
    //se hacen los lets para que retorne al menos un None y cuando ya acaben ambos pasa al join!
    let gen_handle = tokio::spawn(channel_fake_data_creator(tx, timer));
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
                println!("Sequence number: {}", header.sequence_number );
                println!("Timestamp: {}", header.timestamp );
            }
            Err(e)=>{
                eprintln!("Failed to parse header: {:?}", e);
            }
            
        }
    }
    
}


pub async fn advanced_hole_punching(conn: Arc<UdpSocket>,timer: Instant) 
                                        -> Result<(), Box<dyn Error>> {

    println!("Trying to connect to peer. Please wait...");

    let mut interval = time::interval(Duration::from_millis(400));
    let mut buf = [0u8; MAXDATAGRAMSIZE];
    let mut connected = false;

    for i in 1..20 {
        tokio::select! {

            _ = interval.tick() => {
                let ctrl_datagram =dummy_gen::generate_control_datagram(timer);

                conn.send(&ctrl_datagram).await?;

                println!("Connection request number: {i}.");
            }

            result = conn.recv(&mut buf) => {
                let len = result?;
                if len < HEADERSIZE{
                    println!("Wrong header size. Trying again");
                    continue;
                }
                match Header::unserialize(&mut Bytes::copy_from_slice(&buf[..HEADERSIZE])){
                    Ok(header) => {
                        println!("Received {} bytes", len);

                        if header.channel == ChannelType::Sync {
                            println!("Received correct control packet. Initiating communication");
                            connected = true;
                            break;
                        } else {
                            println!("Received packet, but it was not a control packet. Trying again...");
                        }
                    }
                    Err(PacketError::WrongBufferSize) => {
                        println!("Received packet with a wrong size. Trying again.");
                    }

                    Err(error) => {
                        println!("Made an oopsie look: {}", error);
    }
                }
            }
        }
    }
    if connected{
        println!("Sending 5 more packets to stabilize connection");
        let mut interval = time::interval(Duration::from_millis(100));
        for _ in 0..5 {
            interval.tick().await;
            let ctrl_datagram =dummy_gen::generate_control_datagram(timer);
            conn.send(&ctrl_datagram).await?;
            }
        return Ok(());
    }

    Err("Hole punching attempt timed out, no response from peer.".into())
}