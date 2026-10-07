use openh264::{decoder::Decoder, formats::YUVSource};
use std::collections::{BTreeMap};
use bytes::Bytes;


pub fn decode_full_frame(map: &mut BTreeMap<u16, Bytes>) -> Result<(Vec<u8>, usize, usize), Box<dyn std::error::Error>>{
    let mut decoder = Decoder::new().unwrap();
    let mut framevec: Vec<u8> = Vec::new();
    for bytes in map.values() {
        framevec.extend_from_slice(bytes);
    }
    let Ok(Some(yuv)) = decoder.decode(&framevec) else { return Err("Failed to decode frame".into()) };
    let (width, height) = yuv.dimensions();
    let mut rgb_raw = vec![0u8; yuv.rgb8_len()];
    yuv.write_rgb8(&mut rgb_raw);

    Ok((rgb_raw, width, height))
   
}
    