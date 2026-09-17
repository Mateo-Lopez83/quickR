use std::sync::mpsc as enc_mpsc;
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
};
use bytes::Bytes;

use crate::RawStreamData;



pub struct ScreenCapture {
    enc_tx: enc_mpsc::SyncSender<RawStreamData>,
    buffer: Vec<u8>
}

impl GraphicsCaptureApiHandler for ScreenCapture {
    // flag works to get the mpsc sender to the capture handler
    type Flags = enc_mpsc::SyncSender<RawStreamData>;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self { enc_tx: ctx.flags, buffer:Vec::new() })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        //Bytes arrive in BGRA8 format
        let raw_bytes = frame.buffer()?;
        let height = raw_bytes.height() as usize;
        let width = raw_bytes.width() as usize;
        //TODO: Revisar cómo crear esto 1 vez y con cada nonpadding_buffer,
        //vaciarlo y reutilizarlo
        //let mut buffer = Vec::new();
        let payload = Bytes::copy_from_slice(raw_bytes.as_nopadding_buffer(&mut self.buffer));
        let full_payload = RawStreamData {
            data: payload,
            width: width,
            height: height,
        };
        // try_send will drop frames when the queue is full. 
        if self.enc_tx.try_send(full_payload).is_err() {
            //Do nothing if the queue is full
        }

        Ok(())
    }
}

