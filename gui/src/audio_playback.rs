use std::sync::{Arc, atomic::Ordering};

use commons::{AudioChunk, AudioClock, AudioQueue};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const PLAYOUT_DELAY_MS: u32 = 120;

pub fn start_audio_playback(queue: AudioQueue, clock: Arc<AudioClock>) {
    std::thread::spawn(move || {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else { eprintln!("AUD no output device"); return; };
        let config = cpal::StreamConfig { channels: 2, sample_rate: 48_000, buffer_size: cpal::BufferSize::Default };
        let mut current: Option<(AudioChunk, usize)> = None;
        let mut started = false;
        let stream = device.build_output_stream(
            config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut q = queue.lock().unwrap();
                if !started {
                    if (q.len() as u32) * 20 < PLAYOUT_DELAY_MS { out.fill(0.0); return; }
                    started = true;
                }
                let mut i = 0;
                while i < out.len() {
                    if current.is_none() {
                        match q.pop_front() {
                            Some(c) => current = Some((c, 0)),
                            None => { out[i..].fill(0.0); started = false; break; } // underrun: re-buffer
                        }
                    }
                    let (chunk, pos) = current.as_mut().unwrap();
                    let n = (chunk.samples.len() - *pos).min(out.len() - i);
                    out[i..i + n].copy_from_slice(&chunk.samples[*pos..*pos + n]);
                    let played_ms = ((*pos / 2) as u32 * 1000) / 48_000;
                    clock.ts.store(chunk.ts.wrapping_add(played_ms), Ordering::Relaxed);
                    clock.updated_ms.store((commons::wall_us() / 1000) as u64, Ordering::Relaxed);
                    *pos += n; i += n;
                    if *pos >= chunk.samples.len() { current = None; }
                }
            },
            |err| eprintln!("AUD playback error: {err}"),
            None,
        );
        match stream {
            Ok(s) => { let _ = s.play(); loop { std::thread::park(); } } // mantener vivo el stream
            Err(e) => eprintln!("AUD build output: {e}"),
        }
    });
}