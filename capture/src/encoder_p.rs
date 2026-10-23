

pub mod encoder_p{
    use openh264::encoder::{BitRate, Encoder, EncoderConfig, FrameRate, FrameType, IntraFramePeriod, RateControlMode};
    use openh264::formats::{BgraSliceU8, YUVBuffer};
    use tokio::sync::mpsc as udp_mpsc;
    use std::error::Error;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, mpsc as enc_mpsc};
    use std::time::{Duration, Instant};
    use bytes::Bytes;
    use commons::{wall_us, PAYLOADSIZE};
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
    pub fn run_encoder(
            enc_rx: enc_mpsc::Receiver<RawStreamData>, 
            udp_tx: udp_mpsc::Sender<Bytes>, 
            timer: Instant, 
            receiver_idr_request: Arc<AtomicBool>,
            idr_in_flight: Arc<AtomicBool>) {

        let encoder_config = EncoderConfig::new()
            .bitrate(BitRate::from_bps(1_500_000))  //TODO: optimizar el valor de este atributo
            .max_frame_rate(FrameRate::from_hz(30.0))
            .rate_control_mode(RateControlMode::Bitrate)
            .intra_frame_period(IntraFramePeriod::from_num_frames(60)); // hard cap de cada cuanto mandar un IDR
                
        let mut encoder = Encoder::with_api_config(openh264::OpenH264API::from_source(), encoder_config).unwrap();
        let mut sequence_num: u16 = 0;
        let mut last_forced_idr: Option<Instant> = None;
        //si se pierde el IDR ACK se fallbackea a un timeout
        const IDR_ACK_TIMEOUT_MS: u64 = 3000;

        while let Ok(raw_frame) = enc_rx.recv() {
            let t_in = Instant::now(); //solo para logs: mide conversión + encode
            match bytes_to_yub(raw_frame){
                Ok(yubytes) => {
                    if receiver_idr_request.swap(false, Ordering::SeqCst) {
                        let already_in_flight = idr_in_flight.load(Ordering::SeqCst);
                        let timed_out = last_forced_idr.map_or(false, |t| t.elapsed() >= Duration::from_millis(IDR_ACK_TIMEOUT_MS));
                        //si llegó una solicitud de IDR y no se ha mandado ya o ya pasó tiempo desde el ultimo requesr, se manda IDR
                        if !already_in_flight || timed_out {
                            println!("IDR_FORCED in_flight={} timed_out={} wall_us={}", already_in_flight, timed_out, wall_us());
                            encoder.force_intra_frame();
                            idr_in_flight.store(true, Ordering::SeqCst);
                            last_forced_idr = Some(Instant::now());
                        } else {
                            println!("IDR_IGNORED wall_us={}", wall_us());
                        }
                    }
                let encoded = encoder.encode(&yubytes).unwrap();
                let encoded_bytes = Bytes::from(encoded.to_vec());
                let timestamp = timer.elapsed().as_millis() as u32;
                println!("ENC ts={} type={:?} bytes={} frags={} seq_start={} enc_ms={:.2} wall_us={}",
                    timestamp, encoded.frame_type(), encoded_bytes.len(), encoded_bytes.len().div_ceil(PAYLOADSIZE),
                    sequence_num, t_in.elapsed().as_secs_f64() * 1000.0, wall_us());
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
                        match udp_connect::frame_send(true, encoded_bytes, &udp_tx, &mut sequence_num, timestamp){
                            Ok(_) => {},
                            //TODO: mejorar el comportamiento con errores de envio 
                            Err(e) => return ,
                        }
                    }
                    //Skip, Invalid e IPMixed no se envían; el tipo queda en la línea ENC
                    FrameType::Skip => {}
                    FrameType::Invalid => {}
                    FrameType::IPMixed => {}
                }
                //fin del envío al canal; la diferencia con ENC es el tiempo bloqueado en blocking_send
                println!("ENC_QUEUED ts={} wall_us={}", timestamp, wall_us());
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






