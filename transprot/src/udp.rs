use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::Ordering::SeqCst;
use std::time::Instant;
use std::collections::{BTreeMap};
use commons::{FrameInProgress, RGBFrameData, SharedStats};
use commons::MAXDATAGRAMSIZE;
use commons::frame_decoder::decode_btree_frame;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};
use tokio::sync::mpsc::{self};
use bytes::Bytes;
use openh264::decoder::Decoder;
use crate::PacketError;
use commons::header::{ChannelType, FragmentType, HEADERSIZE, Header};
use std::sync::mpsc as frame_mpsc;

pub mod packet_gen{
use std::time::Instant;

use commons::header::{FragmentType, Header, ChannelType};
    use commons::PAYLOADSIZE;
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

    //for advanced hole punching
    pub fn generate_sync_datagram(timer: Instant)->Bytes{
        let time_now = timer.elapsed().as_millis() as u32;

        //por ahora se envía exclusivamente un header sin payload como simulación de SYN/ACK protocl
        generate_header(4, 2, false, 4,true, 0,18,time_now,1).serialize().freeze()
    }
    //usar para solicitar un IDR frame
    pub fn generate_control_datagram(timer: Instant)->Bytes{
        let time_now = timer.elapsed().as_millis() as u32;

        generate_header(4, 2, false, 3,true, 0,18,time_now,1).serialize().freeze()
    }
}

//meant for debugging before capture existed
// async fn channel_fake_data_creator(tx:Sender<Bytes>, timer: Instant){
//     let mut interval = time::interval(Duration::from_millis(1000));

//     let mut seqnum: u16 = 11;
//     for _ in 0..10{
//         interval.tick().await; 
//         let timestamp = timer.elapsed().as_millis() as u32;
//         let fake_payload = packet_gen::generate_fake_datagram(timestamp, seqnum);
//         seqnum = seqnum +1;
//         //println!("{}", String::from_utf8_lossy(&fake_payload));
//         if tx.send(fake_payload).await.is_err() {
//             println!("consumer dropped, stopping generator");
//             break;
//         }
//     }
   
// }

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


async fn idr_request_listener(conn: Arc<UdpSocket>, received_idr_request: Arc<AtomicBool>){
    /*
        Listener that loops forever on the sender to be actively listening for any
        control packets sent by the receiver.
    */
    let mut buf = [0u8; MAXDATAGRAMSIZE];
    let socket = conn.clone();
    loop {
        let result = socket.recv(&mut buf);
        println!("Received something from receiver. Possible IDR request");
            let len = result.await.unwrap();
            let mut received = Bytes::copy_from_slice(&buf[..len]);
            match Header::unserialize(&mut received) {
                Ok(header) => {
                    //por ahora el unico control packet es para pedir IDR
                    //TODO: expandir esto para handlear más control packets
                    if header.channel == ChannelType::Control {
                        println!("Received IDR frame request");
                        received_idr_request.store(true, SeqCst);
                    }
                }
                Err(_) => {
                    println!("Failed to parse header from received packet from client. Ignoring packet.");
                    continue;
                },
            }
    
}
}

pub async fn main_sending_process(conn: Arc<UdpSocket>, timer: Instant) -> Result<(), Box<dyn Error>> {
    let received_idr_request = Arc::new(AtomicBool::new(false));
    //TODO: revisar que tanto importa el tamaño de este buffer
    let (tx, rx) = mpsc::channel::<Bytes>(90);
    let socket = conn.clone();
    let _idr_listener_handle = tokio::spawn(idr_request_listener(conn.clone(), received_idr_request.clone()));
    let _= capture::start_capture(tx, timer, received_idr_request.clone());
    let consumer_handle = tokio::spawn(channel_consumer(rx, socket));

    let _ = consumer_handle.await;
    
    

    Ok(())
}
//____________________________________________________________________

async fn request_idr(conn: Arc<UdpSocket>, timer: Instant) -> Result<(), Box<dyn Error>>{
    let ctrl_datagram =packet_gen::generate_control_datagram(timer);
    conn.send(&ctrl_datagram).await?;
    println!("Sent IDR frame request to peer");
    Ok(())
}

fn drain_ready_frames(saved_frames_map: &mut BTreeMap<u32, FrameInProgress>, decoder: &mut Decoder, timer: Instant,enc_tx: frame_mpsc::SyncSender<RGBFrameData>, last_released: &mut Option<u32>, waiting_for_idr: &mut bool, stats: SharedStats) -> Result<bool, Box<dyn Error>> {
    //returns true if it needs an IDR, either because the decoding failed for some reason or the 
    let mut loop_remove_stales: bool = true;
    let mut should_request_idr: bool = false;
    while loop_remove_stales{
        if let Some((&key, _)) = saved_frames_map.iter().next() {
            let mut should_remove = false;

            if let Some(value) = saved_frames_map.get_mut(&key) {
                let is_complete = value.is_complete;
                let is_stale = (timer.elapsed().as_millis() as u32).wrapping_sub(value.last_updated) > 700;

                //si se necesita el idr y el paquete no lo es, se elimina y se vuelve a intentar
                if *waiting_for_idr && value.is_idr == false {
                    saved_frames_map.remove(&key);
                    continue;
                }
                if is_complete {
                    println!("Attempting to decode frame with timestamp {} and {} packets", key, value.map.len());
                    match decode_btree_frame(&mut value.map, decoder) {
                        Ok(rgb_frame_data) => {
                            *waiting_for_idr = false; 
                            println!("Decoded full fragmented frame with timestamp {} and dimensions {}x{}", key, rgb_frame_data.width, rgb_frame_data.height);

                            //ACÁ SE MANDA EL FRAME AL GUI PARA QUE LO MUESTRE LA GUI
                            match enc_tx.try_send(rgb_frame_data) {
                                Ok(_) => {
                                    println!("Frame with timestamp {} sent to GUI successfully.", key);
                                }
                                Err(e) => {
                                    println!("Failed to send frame with timestamp {} to GUI: {}", key, e);
                                }
                            }
                            should_remove = true;
                        }
                        // Ok(None) => {
                        //     println!("No frame data returned for timestamp {}.", key);
                        //     //should_remove = true;
                        // }
                        Err(e) => {
                            println!("Failed to decode frame with timestamp {}: {}", key, e);
                            should_remove = true; // Remove the frame even if decoding fails
                            should_request_idr = true; // Indicate that an error occurred
                            *waiting_for_idr = true; //will indicate that there's a need for idr until it arrives

                        }
                    }
                    // let rgb_frame_data: RGBFrameData = decode_btree_frame(&mut value.map, decoder)?;
                    // println!("Decoded full fragmented frame with timestamp {} and dimensions {}x{}", key, rgb_frame_data.width, rgb_frame_data.height);
                    // //ACÁ SE MANDA EL FRAME AL GUI PARA QUE LO MUESTRE LA GUI
                    // enc_tx.try_send(rgb_frame_data).map_err(|e| format!("Failed to send frame: {}", e))?;
                    // should_remove = true;
                } else if is_stale {
                    should_remove = true;
                    should_request_idr = true; 
                    
                    *waiting_for_idr = true;
                    
                    
                }
                else{
                    loop_remove_stales = false;
                }
            }

            if should_remove {
                println!("Evicted frame with timestamp {} ", key);
                *last_released = Some(key);
                saved_frames_map.remove(&key);
                stats.frames_dropped.fetch_add(1, Ordering::Relaxed);
                }
        }
        else{
            //map is empty
            loop_remove_stales = false;
        }
    }
    Ok(should_request_idr)
}

pub async fn receiving_process(conn: Arc<UdpSocket>,timer: Instant, enc_tx: frame_mpsc::SyncSender<RGBFrameData>, stats: SharedStats,)->Result<(), Box<dyn Error>>{   
    let mut decoder = Decoder::new().unwrap(); 
    let socket = conn.clone();
    let mut saved_frames_map: BTreeMap<u32, FrameInProgress> = BTreeMap::new();
    let mut first_packet:bool = true;
    let mut curr_timestamp: u32;
    let mut prev_seqnum: u16 = 0;
    let mut actual_seqnum: u16;
    let mut buf = [0u8; MAXDATAGRAMSIZE];
    let mut interval = time::interval(Duration::from_millis(800));
    let mut last_idr_request: Option<Instant> = None;
    let mut needs_idr: bool;
    let mut waiting_for_idr: bool = true;
    let mut last_released: Option<u32> = None;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                needs_idr = drain_ready_frames(&mut saved_frames_map, &mut decoder, timer, enc_tx.clone(), &mut last_released, &mut waiting_for_idr, stats.clone())?;
                //si se necesita el IDR y ha pasado mas de 1 segundo desde el ultimo IDR, se solicita
                if needs_idr || waiting_for_idr {
                            let due = last_idr_request
                                .map_or(true, |t| t.elapsed() >= Duration::from_secs(1));
                            if due {
                                let _ = request_idr(conn.clone(), timer).await;
                                last_idr_request = Some(Instant::now());
                            }
                        }
                
            }
            result = socket.recv(&mut buf) => {
                let len = result?;
                //println!("Received {} bytes from connection",len);
                let mut received = Bytes::copy_from_slice(&buf[..len]);
                match Header::unserialize(&mut received) {
                    Ok(header)=>{
                        stats.packets_received.fetch_add(1, Ordering::Relaxed);
                        //*packets_received+=1;
                        actual_seqnum = header.sequence_number;
                        if first_packet{
                            prev_seqnum = actual_seqnum;
                            first_packet = false;
                        }
                        else if actual_seqnum ==prev_seqnum{
                            //duplicado de paquete, no hacer nada
                            continue;
                        }
                        let seq_difference = (actual_seqnum.wrapping_sub(prev_seqnum) as i16)- 1;
                        if seq_difference > 0 {
                            println!("{seq_difference} packets lost");
                            stats.packets_lost.fetch_add(seq_difference as u64, Ordering::Relaxed);
                            //*packets_lost += seq_difference as u64;
                            waiting_for_idr = true;
                        }      
                        // only advance the marker if this packet is newer
                        if seq_difference >= -1 {
                            prev_seqnum = actual_seqnum;
                        }           

                        //Si se necesita un IDR se loggea. No se ignora el paquete igual
                        // if !header.is_keyframe && needs_idr{
                        //     println!("We are under fucking attack. Non-IDR packet received")
                        // }
                        if header.channel != ChannelType::Video{
                            println!("Received packet with channel {:?}, expected Video. Ignoring packet.", header.channel);
                            continue;
                        }
                        //revisar que el nuevo frame no sea de un timestamp anterior a uno ya decodificado, DIFERENTE A QUE LLEGUE UN PAQUETE DE UN FRAME ANTERIOR
                        if let Some(last) = last_released {
                            if (header.timestamp.wrapping_sub(last) as i32) <= 0 {
                                println!("Dropping late/duplicate packet (ts {}, seq {})", header.timestamp, header.sequence_number);
                                continue;
                            }
                        }
                        

                        curr_timestamp = header.timestamp;
                        actual_seqnum = header.sequence_number;

                        if saved_frames_map.contains_key(&curr_timestamp){
                            
                            let frame_in_progress = saved_frames_map.get_mut(&curr_timestamp).unwrap();
                            frame_in_progress.map.insert(actual_seqnum, received);
                            frame_in_progress.last_updated = timer.elapsed().as_millis() as u32;
                            if header.fragment == FragmentType::Start{
                                frame_in_progress.start_appeared = true;
                                frame_in_progress.start_seqnum = actual_seqnum;
                            }
                            else if header.fragment == FragmentType::End{
                                frame_in_progress.end_appeared = true;
                                frame_in_progress.end_seqnum = actual_seqnum;
                            }
                            if frame_in_progress.start_appeared && frame_in_progress.end_appeared{
                                //frame_in_progress.is_complete = true;
                                let frame_length = (frame_in_progress.end_seqnum.wrapping_sub(frame_in_progress.start_seqnum) as usize) + 1;
                                frame_in_progress.length = frame_length;
                                let tree_size = frame_in_progress.map.len();
                                if tree_size != frame_length{
                                    println!("Frame with timestamp {} has start and end, but has missing packets. Expected length: {}, actual length: {}", curr_timestamp, frame_length, tree_size);
                                    //se espera que lleguen los paquetes faltantes, no se hace nada por ahora
                                }
                                else{
                                    println!("Frame with timestamp {} is complete and has all packets. Length: {}", curr_timestamp, frame_length);
                                    frame_in_progress.is_complete = true;
                                }
                                
                            }
                        }
                        else{
                            let mut new_frame = FrameInProgress {
                                map: BTreeMap::new(),
                                start_seqnum: actual_seqnum,
                                end_seqnum: actual_seqnum,
                                start_appeared: false,
                                end_appeared: false,
                                length: 0,
                                is_complete: if FragmentType::Unfragmented == header.fragment { true } else { false },
                                receiver_timestamp: timer.elapsed().as_millis() as u32,
                                is_idr: header.is_keyframe,
                                last_updated: timer.elapsed().as_millis() as u32,
                            };
                            new_frame.map.insert(actual_seqnum, received);
                            if header.fragment == FragmentType::Start{
                                new_frame.start_appeared = true;
                                new_frame.start_seqnum = actual_seqnum;
                            }
                            else if header.fragment == FragmentType::End{
                                new_frame.end_appeared = true;
                                new_frame.end_seqnum = actual_seqnum;
                            }
                            saved_frames_map.insert(curr_timestamp, new_frame);
                        }
                        needs_idr = drain_ready_frames(&mut saved_frames_map, &mut decoder, timer, enc_tx.clone(), &mut last_released, &mut waiting_for_idr, stats.clone())?;
                        if needs_idr || waiting_for_idr {
                            let due = last_idr_request
                                .map_or(true, |t| t.elapsed() >= Duration::from_secs(1));
                            if due {
                                //println!("Enviando request IDR");
                                let _ = request_idr(conn.clone(), timer).await;
                                last_idr_request = Some(Instant::now());
                            }
                        }
                    }
                    Err(e)=>{
                        eprintln!("Failed to parse header: {:?}", e);
                    }
                    
                }
            }
        }
    }
}

                
                //println!("Got header from {addr}: {:?}", header);
                // println!("Payload length: {}", received.len());
                // println!("Sequence number: {}", header.sequence_number );
                // println!("Timestamp: {}", header.timestamp );
                // actual_seqnum = header.sequence_number;
                //logica para el primer paquete recibido
                // if !first_packet{
                //     prev_seqnum = header.sequence_number;
                //     first_packet = true;
                // }
                // else if actual_seqnum ==prev_seqnum{
                //     //duplicado de paquete, no hacer nada
                //     continue;
                // }
                // let seq_difference = (actual_seqnum.wrapping_sub(prev_seqnum) as i16)- 1;
                // if seq_difference > 0 {
                //     println!("{seq_difference} packets lost");
                //     packets_lost += seq_difference as u32;

                // } else if seq_difference < -1 {
                //     println!("Late packet");
                // }

                // // only advance the marker if this packet is newer
                // if seq_difference >= -1 {
                //     prev_seqnum = actual_seqnum;
                // }    
                       
                
                // //decode

                // println!("Packets lost until now: {packets_lost}");
                // println!("Packets received successfully: {packets_received}");
                // if header.fragment== FragmentType::Start{
                //     curr_timestamp = header.timestamp;
                //     println!("Comienzo del frame numero {curr_timestamp} ");
                // }
                // else if header.fragment== FragmentType::End && curr_timestamp== header.timestamp{
                //     curr_timestamp = header.timestamp;
                //     println!("Se acabó el frame numero {curr_timestamp} con {paquetes_inbetween} paquetes Middle ");
                //     paquetes_inbetween = 0;
                // }
                // else if header.fragment== FragmentType::Middle && curr_timestamp== header.timestamp{
                //     paquetes_inbetween +=1
                // }






                
pub async fn advanced_hole_punching(conn: Arc<UdpSocket>,timer: Instant) 
                                        -> Result<(), Box<dyn Error>> {

    println!("Trying to connect to peer. Please wait...");

    let mut interval = time::interval(Duration::from_millis(400));
    let mut buf = [0u8; MAXDATAGRAMSIZE];
    let mut connected = false;

    for i in 1..20 {
        tokio::select! {

            _ = interval.tick() => {
                let ctrl_datagram =packet_gen::generate_sync_datagram(timer);

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
                        //println!("Received {} bytes", len);

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
            let ctrl_datagram =packet_gen::generate_sync_datagram(timer);
            conn.send(&ctrl_datagram).await?;
            }
        return Ok(());
    }
    
    Err("Hole punching attempt timed out, no response from peer.".into())
}