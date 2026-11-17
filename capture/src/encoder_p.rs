

pub mod encoder_p{
    use openh264::{Timestamp, encoder::{BitRate, Encoder, EncoderConfig, FrameRate, FrameType, IntraFramePeriod, RateControlMode}, formats::YUVSource};
    use openh264::formats::{BgraSliceU8, YUVBuffer};
    use tokio::sync::mpsc as udp_mpsc;
    use std::error::Error;
    use std::sync::atomic::{AtomicBool, Ordering, AtomicU32};
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

    // Downscaling del BGRA a traves de metodo de nearest neighbor average
    fn downscale_bgra(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
        let mut dst = vec![0u8; dw * dh * 4];
        let x_map: Vec<usize> = (0..dw).map(|x| (x * sw / dw) * 4).collect();
        for y in 0..dh {
            let src_row = &src[(y * sh / dh) * sw * 4..][..sw * 4];
            let dst_row = &mut dst[y * dw * 4..][..dw * 4];
            for (x, &sx) in x_map.iter().enumerate() {
                dst_row[x * 4..x * 4 + 4].copy_from_slice(&src_row[sx..sx + 4]);
            }
        }
        dst
    }


    

    //rx recibe de windows-capture, envía a tx el encoded bytestream
    pub fn run_encoder(
            enc_rx: enc_mpsc::Receiver<RawStreamData>, 
            udp_tx: udp_mpsc::Sender<Bytes>, 
            timer: Instant, 
            receiver_idr_request: Arc<AtomicBool>,
            idr_in_flight: Arc<AtomicBool>,
            target_height: Arc<AtomicU32>) {

        let encoder_config = EncoderConfig::new()
            .bitrate(BitRate::from_bps(1_500_000))  //TODO: optimizar el valor de este atributo
            .max_frame_rate(FrameRate::from_hz(30.0))
            .rate_control_mode(RateControlMode::Bitrate)
            .complexity(openh264::encoder::Complexity::Low)
            //.usage_type(openh264::encoder::UsageType::ScreenContentRealTime)
            .intra_frame_period(IntraFramePeriod::from_num_frames(60)); // hard cap de cada cuanto mandar un IDR
                
        let mut encoder = Encoder::with_api_config(openh264::OpenH264API::from_source(), encoder_config).unwrap();
        let mut sequence_num: u16 = 0;
        let mut last_forced_idr: Option<Instant> = None;
        //si se pierde el IDR ACK se fallbackea a un timeout
        const IDR_ACK_TIMEOUT_MS: u64 = 3000;

        while let Ok(raw_frame) = enc_rx.recv() {
            let t_in = Instant::now(); //solo para logs: mide conversión + encode
            let target_h = target_height.load(Ordering::Relaxed) as usize;
            let raw_frame = if target_h != 0 && target_h < raw_frame.height {
                let dh = target_h & !1;                                   // I420 necesita dimensiones pares
                let dw = (raw_frame.width * dh / raw_frame.height) & !1;  // mantiene la proporción
                let data = downscale_bgra(&raw_frame.data, raw_frame.width, raw_frame.height, dw, dh);
                RawStreamData { data: Bytes::from(data), width: dw, height: dh }
            } else {
                raw_frame
            };
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
                //let encoded = encoder.encode(&yubytes).unwrap();
                let timestamp = timer.elapsed().as_millis() as u32;
                let encoded = encoder.encode_at(&yubytes, Timestamp::from_millis(timestamp as u64)).unwrap();
                
                let encoded_bytes = Bytes::from(encoded.to_vec());
                
                println!("ENC ts={} type={:?} bytes={} frags={} seq_start={} enc_ms={:.2} wall_us={} w={} h={}",
                timestamp, encoded.frame_type(), encoded_bytes.len(), encoded_bytes.len().div_ceil(PAYLOADSIZE),
                sequence_num, t_in.elapsed().as_secs_f64() * 1000.0, wall_us(), yubytes.dimensions().0, yubytes.dimensions().1);

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






