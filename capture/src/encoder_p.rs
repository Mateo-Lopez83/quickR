

pub mod encoder_p{
    use tokio::sync::mpsc as udp_mpsc;
    use std::error::Error;
    use std::sync::mpsc as enc_mpsc;
    use bytes::Bytes;
    use openh264::decoder::Decoder;
    use openh264::{nal_units};
    //rx recibe de windows-capture, envía a tx el encoded bytestream
    pub fn run_encoder(enc_rx: enc_mpsc::Receiver<Bytes>, udp_tx: udp_mpsc::Sender<Bytes>) {
        while let Ok(raw_frame) = enc_rx.recv() {
            // TODO: BGRA -> YUV conversion, then encoder.encode(&yuv)
            let encoded = raw_frame; // placeholder until the real encode call is wired in
            
            if udp_tx.blocking_send(encoded).is_err() {
                break; // udp pipeline gone, stop encoding
            }
        }
    }
    pub fn encode_example()-> Result<(), Box<dyn Error>>{
        let h264_in = include_bytes!("../data/multi_512x512.h264");
        let mut decoder = Decoder::new()?;

        // Split H.264 into NAL units and decode each.
        for packet in nal_units(h264_in) {
            // On the first few frames this may fail, so you should check the result
            // a few packets before giving up.
            let maybe_some_yuv = decoder.decode(packet);
        }
        Ok(())
    }
}






