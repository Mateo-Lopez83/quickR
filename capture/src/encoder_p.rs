use openh264::decoder::Decoder;
use openh264::{nal_units};
use std::error::Error;


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

