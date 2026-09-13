//! Step 3: prep for your RFC 6184 (H.264-over-RTP) packetizer.
//!
//! This isn't a codec test anymore — it confirms you can reliably pull
//! individual NAL units (SPS, PPS, IDR slice, ...) out of real encoder
//! output, which is exactly the shape of data your RTP payloader needs to
//! consume one unit at a time.

use openh264::decoder::{DecoderConfig,Decoder};
use openh264::encoder::{Encoder, EncoderConfig, FrameType};
use openh264::formats::{YUVBuffer, YUVSource};
use openh264::{Error, OpenH264API, nal_units};





#[test]
fn decoder_initializes() -> Result<(), Error> {
    let api = OpenH264API::from_source();
    let config = DecoderConfig::default();
    let _decoder = Decoder::with_api_config(api, config)?;
    Ok(())
}
 
#[test]
fn encoder_initializes() -> Result<(), Error> {
    let api = OpenH264API::from_source();
    let config = EncoderConfig::new();
    let _encoder = Encoder::with_api_config(api, config)?;
    Ok(())
}
 
#[test]
fn default_constructors_work_too() -> Result<(), Error> {
    
    let _decoder = Decoder::new()?;
    let _encoder = Encoder::new()?;
    Ok(())
}


#[test]
fn first_frame_yields_sps_pps_and_idr_layers() -> Result<(), Error> {
    let (width, height) = (64usize, 48usize);
    let yuv = YUVBuffer::new(width, height);

    let api = OpenH264API::from_source();
    let mut encoder = Encoder::with_api_config(api, EncoderConfig::new())?;
    let stream = encoder.encode(&yuv)?;

    // The first encoded frame is always an IDR and typically arrives as two
    // layers: layer 0 = non-video (SPS + PPS), layer 1 = video (the slice).
    assert_eq!(stream.frame_type(), FrameType::IDR);
    assert!(stream.num_layers() >= 2);

    let param_layer = stream.layer(0).expect("layer 0 should exist");
    assert!(!param_layer.is_video(), "layer 0 should be SPS/PPS, not video");

    let video_layer = stream.layer(1).expect("layer 1 should exist");
    assert!(video_layer.is_video(), "layer 1 should be the video slice");

    Ok(())
}

#[test]
fn nal_units_splits_flattened_bitstream_correctly() -> Result<(), Error> {
    let (width, height) = (64usize, 48usize);
    let yuv = YUVBuffer::new(width, height);

    let mut encoder = Encoder::new()?;
    let flat: Vec<u8> = encoder.encode(&yuv)?.to_vec();

    // This mirrors what nal_units() does internally: splitting on
    // 00 00 01 / 00 00 00 01 start codes. Each returned slice is one
    // complete NAL unit 
    let units: Vec<&[u8]> = nal_units(&flat).collect();

    assert!(
        units.len() >= 3,
        "expected at least SPS + PPS + one slice NAL, got {}",
        units.len()
    );

    // Every NAL unit must start with a start code prefix, and have a non-empty payload after it
    for unit in &units {
        assert!(
            unit.starts_with(&[0, 0, 0, 1]) || unit.starts_with(&[0, 0, 1]),
            "NAL unit missing a start code prefix"
        );
        assert!(unit.len() > 4, "NAL unit has no payload beyond its start code");
    }

    Ok(())
}


#[test]
fn encode_then_decode_black_frame() -> Result<(), Error> {
    // Dimensions must be even (I420 chroma subsampling).
    let (width, height) = (64usize, 48usize);
    let yuv_in = YUVBuffer::new(width, height);
 
    let api = OpenH264API::from_source();
    let mut encoder = Encoder::with_api_config(api, EncoderConfig::new())?;
    let bitstream = encoder.encode(&yuv_in)?;
    let encoded: Vec<u8> = bitstream.to_vec();
 
    assert!(
        !encoded.is_empty(),
        "encoder produced no bytes at all — codec is not healthy on this device"
    );
 
    let api = OpenH264API::from_source();
    let mut decoder = Decoder::with_api_config(api, DecoderConfig::default())?;
 
    // A single call to decode() is enough for this tiny stream since the
    // encoder emits SPS/PPS + one IDR frame together for the first frame.
    // For multi-frame streams you'd feed each NAL unit (see nal_fragments.rs).
    let decoded = decoder
        .decode(&encoded)?
        .ok_or_else(|| Error::msg("decoder returned no frame for a stream that should have one"))?;
 
    assert_eq!(
        decoded.dimensions(),
        (width, height),
        "decoded frame dimensions do not match what was encoded"
    );
 
    Ok(())
}
 
#[test]
fn encode_then_decode_larger_frame() -> Result<(), Error> {
    // Slightly bigger, non-multiple-of-16 dims to catch stride/padding bugs
    // early rather than only testing "nice" resolutions.
    let (width, height) = (130usize, 98usize);
    let yuv_in = YUVBuffer::new(width, height);
 
    let mut encoder = Encoder::new()?;
    let encoded = encoder.encode(&yuv_in)?.to_vec();
 
    let mut decoder = Decoder::new()?;
    let decoded = decoder
        .decode(&encoded)?
        .ok_or_else(|| Error::msg("decoder returned no frame"))?;
 
    assert_eq!(decoded.dimensions(), (width, height));
 
    Ok(())
}