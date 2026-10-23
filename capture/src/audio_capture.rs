use bytes::{BufMut, Bytes, BytesMut};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{sync::mpsc, time::Instant};
use tokio::sync::mpsc::Sender;
use commons::header::{ChannelType, FragmentType, Header};
use opus::Encoder;
const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: usize = 2;
const FRAME_MS: usize = 20;
const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE as usize / 1000) * FRAME_MS; // 960 per channel

pub fn start_audio_capture(
    udp_tx: Sender<Bytes>,
    clock_start: Instant, //mismo que el video capture
) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no output device")?;
    let config = device.default_output_config()?;
    // Assumes the device mix format is 48kHz stereo f32; check config.sample_rate()
    // and config.channels() and add rubato/downmixing if it isn't.

    let (pcm_tx, pcm_rx) = mpsc::channel::<Vec<f32>>();

    let stream = device.build_input_stream(
        config.into(),
        move |data: &[f32], _| {
            let _ = pcm_tx.send(data.to_vec()); 
        },
        |err| eprintln!("loopback error: {err}"),
        None,
    )?;
    stream.play()?;

    // Encoder
    std::thread::spawn(move || {
        let mut encoder = Encoder::new(
            SAMPLE_RATE,
            opus::Channels::Stereo,
            opus::Application::Audio,
        )
        .expect("opus init");

        let frame_len = SAMPLES_PER_FRAME * CHANNELS;
        let mut pending: Vec<f32> = Vec::with_capacity(frame_len * 2);
        let mut out_buf = [0u8; 4000]; // max Opus packet size
        let mut seq: u32 = 0;

        while let Ok(chunk) = pcm_rx.recv() {
            pending.extend_from_slice(&chunk);

            // Emit as many complete 20ms frames as we have
            while pending.len() >= frame_len {
                let frame: Vec<f32> = pending.drain(..frame_len).collect();
                let n = match encoder.encode_float(&frame, &mut out_buf) {
                    Ok(n) => n,
                    Err(e) => { eprintln!("opus encode: {e}"); continue; }
                };
                
                let audio_header = Header::new(
                    FragmentType::Unfragmented,
                    2,
                    false,
                    ChannelType::Audio,
                    false,
                    seq as u16,
                    n as u16,
                     clock_start.elapsed().as_millis() as u32, 2
                ).serialize();
                let mut combined = BytesMut::with_capacity(audio_header.len() + n);
                combined.put_slice(&audio_header);
                combined.put_slice(&out_buf[..n]);

                seq = seq.wrapping_add(1);

                if udp_tx.try_send(combined.freeze()).is_err() {
                    return; // receiver dropped, shut down
                }
            }
        }
    });

    Ok(stream)
}