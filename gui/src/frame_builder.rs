use openh264::formats::YUVSource;
use egui::{ColorImage, TextureHandle, TextureOptions};
use std::sync::mpsc;

fn receive_frame_buffer(rx: &mpsc::Receiver<(Vec<u8>, usize, usize)>) -> Option<ColorImage> {
    if let Ok((frame, width, height)) = rx.try_recv() {
        Some(transform_yuv_to_rgb(frame, width, height))
    } else {
        None
    }

}

fn transform_yuv_to_rgb(frame: Vec<u8>, width: usize, height: usize) -> ColorImage {
    ColorImage::from_rgb(
        [width as usize, height as usize],
        &frame,
    )
}