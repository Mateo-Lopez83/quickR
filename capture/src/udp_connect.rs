use std::error::Error;
use bytes::{Bytes, BytesMut, BufMut};
use tokio::sync::mpsc as udp_mpsc;
use commons::PAYLOADSIZE;
use commons::header::{Header,FragmentType,ChannelType};


pub fn frame_send(is_keyframe:bool, payload: Bytes, udp_tx: &udp_mpsc::Sender<Bytes>, seqnum: &mut u16, timestamp: u32)-> Result<(), Box<dyn Error>>{
    
    let chunks: Vec<&[u8]> = payload.chunks(PAYLOADSIZE).collect();
    let total_chunks = chunks.len();
    for (i, chunk) in chunks.iter().enumerate() {
        let fragment_type = if total_chunks == 1 {
            //caso en el que no sea un big payload
            FragmentType::Unfragmented
        } else if i == 0 {
            FragmentType::Start
        } else if i == total_chunks - 1 {
            FragmentType::End
        } else {
            FragmentType::Middle
        };
        let fragment_header = Header::new(
        fragment_type,
        1,
        false,
        ChannelType::Video,
        is_keyframe,
        *seqnum,
        chunk.len() as u16,
        timestamp, // same value for every fragment of this frame
        1, //TODO: cambiar esto cuando ya se implemente el ssrc check
        ).serialize();
        let mut combined = BytesMut::with_capacity(fragment_header.len() + chunk.len());
        combined.put_slice(&fragment_header);
        combined.put_slice(&chunk);
        if udp_tx.blocking_send(combined.freeze()).is_err(){
            println!("udp_rx failed. Stopping encoding pipeline");
            break;
        }
        *seqnum +=1;

    }
        


    Ok(())


}