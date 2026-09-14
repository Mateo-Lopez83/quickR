use std::sync::mpsc as enc_mpsc;
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
};
use bytes::Bytes;



pub struct ScreenCapture {
    enc_tx: enc_mpsc::SyncSender<Bytes>,
}

impl GraphicsCaptureApiHandler for ScreenCapture {
    // flag works to get the mpsc sender to the capture handler
    type Flags = enc_mpsc::SyncSender<Bytes>;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self { enc_tx: ctx.flags })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        //Bytes arrive in BGRA8 format
        let mut raw_bytes = frame.buffer()?;
        let payload = Bytes::copy_from_slice(raw_bytes.as_raw_buffer());

        // try_send will drop frames when the queue is full. 
        if self.enc_tx.try_send(payload).is_err() {
            //Do nothing if the queue is full
        }

        Ok(())
    }
}

