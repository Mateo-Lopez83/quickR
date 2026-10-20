

pub mod encoder_p{
    use openh264::encoder::{Encoder, FrameType};
    use openh264::formats::{BgraSliceU8, YUVBuffer};
    use tokio::sync::mpsc as udp_mpsc;
    use std::error::Error;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, mpsc as enc_mpsc};
    use std::time::{Instant};
    use bytes::Bytes;    
    use crate::{RawStreamData, udp_connect};

    
    fn bytes_to_yub(bytedata: RawStreamData)-> Result<YUVBuffer, Box<dyn Error>>{
        let height = bytedata.height;
        let width = bytedata.width;
        if bytedata.data.len() != height * width * 4{
            return Err("Invalid BGRA buffer size".into());
        }
        let bgra = BgraSliceU8::new(&bytedata.data, (width, height));
        Ok(YUVBuffer::from_bgra8_source(bgra))
    }


    //rx recibe de windows-capture, envía a tx el encoded bytestream
    pub fn run_encoder(enc_rx: enc_mpsc::Receiver<RawStreamData>, udp_tx: udp_mpsc::Sender<Bytes>, timer: Instant, receiver_idr_request: Arc<AtomicBool> ) {
        let mut encoder = Encoder::new().unwrap();
        let mut sequence_num: u16 = 0;
        while let Ok(raw_frame) = enc_rx.recv() {
            match bytes_to_yub(raw_frame){
                Ok(yubytes) => {
                    if receiver_idr_request.swap(false,Ordering::SeqCst) {
                        println!("Sending IDR frame due to request");
                        encoder.force_intra_frame();
                    }
                let encoded = encoder.encode(&yubytes).unwrap();
                let encoded_bytes = Bytes::from(encoded.to_vec());
                let timestamp = timer.elapsed().as_millis() as u32;
                match encoded.frame_type() {
                    FrameType::I => {
                        match udp_connect::frame_send(false, encoded_bytes, &udp_tx, &mut sequence_num, timestamp){
                            Ok(_) => {},
                            //TODO: mejorar el comportamiento con errores de envio 
                            Err(e) => return,
                        }
                    }
                    FrameType::P => {
                        match udp_connect::frame_send(false, encoded_bytes, &udp_tx, &mut sequence_num, timestamp){
                            Ok(_) => {},
                            //TODO: mejorar el comportamiento con errores de envio 
                            Err(e) => return ,
                        }
                    }
                    FrameType::IDR => {
                        println!("_____IDR frame created. Sending frame______");
                        match udp_connect::frame_send(true, encoded_bytes, &udp_tx, &mut sequence_num, timestamp){
                            Ok(_) => {},
                            //TODO: mejorar el comportamiento con errores de envio 
                            Err(e) => return ,
                        }
                    }
                    FrameType::Skip => {
                        println!("_____Skip frame created. Not sent______");
                    }
                    FrameType::Invalid => {
                        println!("_____Invalid frame created. Not sent______");
                    }
                    FrameType::IPMixed => {
                        println!("_____IPMixed frame created. Not sent______");
                    }
                }
                },
                Err(_) => {
                    println!("algo raro con la conversión de bytes a BGRA a YUB");
                    //just try again with the next frame
                    continue
                },
            } 
        }
        println!("Encoder thread finished, no more frames to encode");
    }
    // pub fn encode_example()-> Result<(), Box<dyn Error>>{
    //     let h264_in = include_bytes!("../data/multi_512x512.h264");
    //     let mut decoder = Decoder::new()?;
    //     let mut encoder = Encoder::new();

    //     // Split H.264 into NAL units and decode each.
    //     for packet in nal_units(h264_in) {
    //         // On the first few frames this may fail, so you should check the result
    //         // a few packets before giving up.
    //         let maybe_some_yuv = decoder.decode(packet);
    //     }
    //     Ok(())
    // }
}






