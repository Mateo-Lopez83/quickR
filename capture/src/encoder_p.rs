pub mod encoder_p{
    use openh264::encoder::{Encoder, FrameType};
    use openh264::formats::{BgrSliceU8, BgraSliceU8, YUVBuffer};
use tokio::sync::mpsc as udp_mpsc;
//use windows_capture::encoder::VideoSettingsSubType::BGRA8;
    use std::error::Error;
    use std::sync::mpsc as enc_mpsc;
use std::time::Instant;
    use bytes::Bytes;
    //use openh264::decoder::Decoder;
    
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
    pub fn run_encoder(enc_rx: enc_mpsc::Receiver<RawStreamData>, udp_tx: udp_mpsc::Sender<Bytes>, timer: Instant) {
        let mut encoder = Encoder::new().unwrap();
        let mut sequence_num: u16 = 0;
        while let Ok(raw_frame) = enc_rx.recv() {
            // TODO: BGRA -> YUV conversion, then encoder.encode(&yuv)
            match bytes_to_yub(raw_frame){
                Ok(yubytes) => {
                let encoded = encoder.encode(&yubytes).unwrap();
                let encoded_bytes = Bytes::from(encoded.to_vec());
                let timestamp = timer.elapsed().as_millis() as u32;
                match encoded.frame_type() {
                    FrameType::I => {
                        match udp_connect::frame_send(true, encoded_bytes, &udp_tx, &mut sequence_num, timestamp){
                            Ok(_) => todo!(),
                            Err(_) => todo!(),
                        }
                    }
                    FrameType::P => {
                        match udp_connect::frame_send(true, encoded_bytes, &udp_tx, &mut sequence_num, timestamp){
                            Ok(_) => todo!(),
                            Err(_) => todo!(),
                        }
                    }
                    _ => {
                        println!("_____ERROR defining keyframe-ness of frame______");
                    }
                }
                
                // if udp_tx.blocking_send(encoded_bytes).is_err(){
                //     println!("udp_rx failed. Stopping encoding pipeline");
                //     break;
                // }
                },
                Err(_) => {
                    println!("algo raro con la conversión de BGRA a YUB");
                    continue
                },
            } // placeholder until the real encode call is wired in
            
            // if udp_tx.blocking_send(encoded).is_err() {
            //     break; 
            // }
        }
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






