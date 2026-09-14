use std::sync::mpsc;
use bytes::Bytes;

pub fn main_capture_process(){
    let (tx, rx) = mpsc::channel::<Bytes>();

}