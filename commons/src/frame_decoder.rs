use openh264::{decoder::Decoder, formats::YUVSource};
use std::collections::{BTreeMap};
use bytes::Bytes;
use crate::RGBFrameData;


pub fn decode_btree_frame(map: &mut BTreeMap<u16, Bytes>, decoder: &mut Decoder) -> Result<RGBFrameData, Box<dyn std::error::Error>>{
    let mut framevec: Vec<u8> = Vec::new();
    for bytes in map.values() {
        framevec.extend_from_slice(bytes);
    }
    println!("Decoding frame of size: {}", framevec.len());
    match decoder.decode(&framevec) {
        Ok(Some(yuv)) => {
            let (width, height) = yuv.dimensions();
            let mut rgb_raw = vec![0u8; yuv.rgb8_len()];
            yuv.write_rgb8(&mut rgb_raw);
            Ok(RGBFrameData { data: rgb_raw, width, height })
        }
        Ok(None) => Err("No frame data returned".into()),
        Err(e) => Err(format!("Failed to decode frame: {}", e).into()),
    }
    // let Ok(Some(yuv)) = decoder.decode(&framevec) 
    //     else { 
    //         return Err("Failed to decode frame".into()) 
    //     };
    // let (width, height) = yuv.dimensions();
    // let mut rgb_raw = vec![0u8; yuv.rgb8_len()];
    // yuv.write_rgb8(&mut rgb_raw);

    // Ok(RGBFrameData { data: rgb_raw, width, height })
   
}

    