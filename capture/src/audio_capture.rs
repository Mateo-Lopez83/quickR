use bytes::{BufMut, Bytes, BytesMut};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{sync::mpsc, time::Instant};
use tokio::sync::mpsc::{Sender, error::TrySendError};
use commons::{header::{ChannelType, FragmentType, Header, HEADERSIZE}, PAYLOADSIZE};
use opus::{Bitrate, Encoder};

//pueden cambiar con el tiempo
const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: usize = 2;
const FRAME_MS: usize = 20;
const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE as usize / 1000) * FRAME_MS;




pub fn start_audio_capture(udp_tx: Sender<Bytes>, clock_start: Instant) {
    std::thread::spawn(move || {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else { eprintln!("AUD no output device"); return; };
        let supported = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => { eprintln!("AUD config error: {e}"); return; }
        };
        let in_rate = supported.sample_rate();
        let in_ch = supported.channels() as usize;
        println!("AUD device rate={} ch={} fmt={:?}", in_rate, in_ch, supported.sample_format());
        // Opus solo acepta 48k aquí; si el dispositivo está en 44.1k, cambiarlo en Sonido > Avanzado
        if in_rate != SAMPLE_RATE || supported.sample_format() != cpal::SampleFormat::F32 {
            eprintln!("AUD formato no soportado, audio deshabilitado");
            return;
        }

        let (pcm_tx, pcm_rx) = mpsc::channel::<(u32, Vec<f32>)>();
        let stream = match device.build_input_stream(
            supported.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // ts de llegada del chunk (último sample)
                let _ = pcm_tx.send((clock_start.elapsed().as_millis() as u32, data.to_vec()));
            },
            |err| eprintln!("AUD loopback error: {err}"),
            None,
        ) {
            Ok(s) => s,
            Err(e) => { eprintln!("AUD build stream: {e}"); return; }
        };
        if let Err(e) = stream.play() { eprintln!("AUD play: {e}"); return; }

        let mut encoder = Encoder::new(SAMPLE_RATE, opus::Channels::Stereo, opus::Application::Audio).expect("opus init");
        let _ = encoder.set_bitrate(Bitrate::Bits(64_000)); // ~160 B por paquete de 20 ms
        let _ = encoder.set_inband_fec(true);
        let _ = encoder.set_packet_loss_perc(5);

        let frame_len = SAMPLES_PER_FRAME * CHANNELS;
        let mut pending: Vec<f32> = Vec::with_capacity(frame_len * 4);
        let mut pending_start_ts: u32 = 0; // ts del primer sample en `pending`
        let mut out_buf = [0u8; PAYLOADSIZE]; // garantiza que el paquete cabe en un datagrama
        let mut seq: u16 = 0;

        while let Ok((arrival_ts, chunk)) = pcm_rx.recv() {
            // a estéreo (mono -> duplicar, >2 canales -> primeros dos)
            let chunk_frames = chunk.len() / in_ch;
            if pending.is_empty() {
                // re-sincroniza tras silencios (loopback no entrega nada en silencio)
                pending_start_ts = arrival_ts.wrapping_sub((chunk_frames as u32 * 1000) / in_rate);
            }
            for f in chunk.chunks_exact(in_ch) {
                let (l, r) = if in_ch == 1 { (f[0], f[0]) } else { (f[0], f[1]) };
                pending.push(l);
                pending.push(r);
            }
            while pending.len() >= frame_len {
                let n = match encoder.encode_float(&pending[..frame_len], &mut out_buf) {
                    Ok(n) => n,
                    Err(e) => { eprintln!("AUD opus encode: {e}"); pending.drain(..frame_len); continue; }
                };
                pending.drain(..frame_len);
                let ts = pending_start_ts;
                pending_start_ts = pending_start_ts.wrapping_add(FRAME_MS as u32);

                let header = Header::new(FragmentType::Unfragmented, 2, false, ChannelType::Audio,
                                         false, seq, n as u16, ts, 2).serialize();
                let mut combined = BytesMut::with_capacity(header.len() + n);
                combined.put_slice(&header);
                combined.put_slice(&out_buf[..n]);
                seq = seq.wrapping_add(1);

                match udp_tx.try_send(combined.freeze()) {
                    Ok(()) => {}
                    Err(TrySendError::Full(_)) => println!("AUD drop_full ts={}", ts),
                    Err(TrySendError::Closed(_)) => return,
                }
            }
        }
        drop(stream);
    });
}