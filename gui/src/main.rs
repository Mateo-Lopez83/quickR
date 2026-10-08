#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example
mod frame_builder;
use std::{env, error::Error, net::SocketAddr, sync::Arc, time::{Duration, Instant}};
use commons::RGBFrameData;
use eframe::egui;
use egui::{ColorImage, frame};
use tokio::{net::UdpSocket, runtime::Runtime};
//use tokio::time::sleep;
use std::sync::mpsc as frame_mpsc;
use transprot::{
    udp::{self, receiving_process,advanced_hole_punching}, 
    stuntry::discover_public_address
};
use commons::ip_converter::{turn_code_to_ip, turn_ip_to_code};
struct MyApp {
    own_code: String,
    remote_addr: String,
    connecting: bool,
    error: Option<String>,
    runtime: Arc<Runtime>,
    conn: Arc<UdpSocket>,
    status_rx: std::sync::mpsc::Receiver<Result<(), String>>,
    status_tx: std::sync::mpsc::Sender<Result<(), String>>,
    frame_tx: frame_mpsc::SyncSender<RGBFrameData>,
    frame_rx: frame_mpsc::Receiver<RGBFrameData>,
    connected: bool,
    devtype: String,
    timer: Instant,
    texture: Option<egui::TextureHandle>

}
impl MyApp {
    fn new(runtime: Arc<Runtime>,conn: Arc<UdpSocket>,own_code: String,frame_tx: frame_mpsc::SyncSender<RGBFrameData>, devtype: String, frame_rx: frame_mpsc::Receiver<RGBFrameData>, timer: Instant
    ) -> Self {
        let (status_tx, status_rx) = std::sync::mpsc::channel();

        Self {
            own_code,
            remote_addr: String::new(),
            connecting: false,
            error: None,
            runtime,
            conn,
            status_rx,
            status_tx,
            frame_tx,
            connected: false,
            devtype,
            frame_rx,
            timer,
            texture: None

        }
    }
}



async fn udp_main_process(frame_tx: frame_mpsc::SyncSender<RGBFrameData>, 
                          devtype: String, 
                          //runtime: Arc<Runtime>, 
                          conn: Arc<UdpSocket>, 
                          timer: Instant) {
    match devtype.as_str() {
        "send"=>{
            let _ = udp::main_sending_process(conn, timer).await;
        }
        "receive"=>{
            let _ = receiving_process(conn, timer, frame_tx).await;
        }
        othe =>{
            println!("No command associated with {}",othe);
        } 
    
    }


}

async fn get_stun_ip(conn: Arc<UdpSocket>) -> String {
    let stun_server = "stun4.l.google.com:19302";
    let public_ip = discover_public_address(stun_server, conn.clone()).await.expect("Failed to discover public address");
    let ip_string = turn_ip_to_code(&public_ip.ip, &public_ip.port);
    println!("Your code is: {}", &ip_string);
    ip_string
}


fn main() -> eframe::Result {
    let runtime = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
    let (enc_tx, enc_rx) = frame_mpsc::sync_channel::<RGBFrameData>(1);
    //let video_ssrc: u32 = rng.random();
    //let audio_ssrc: u32 = rng.random();
    let mut timer: Instant = Instant::now();
    let devtype:String = env::args().nth(1).expect("you have to specify if this is a sender or receiver");
    let conn = Arc::new(runtime.block_on(UdpSocket::bind("0.0.0.0:0")).unwrap());
    let ip_string = runtime.block_on(get_stun_ip(conn.clone()));
    //se envía al GUI para que tenga conexión al resto de vainas
    let app = MyApp::new(runtime.clone(), conn.clone(), ip_string, enc_tx.clone(), devtype.clone(), enc_rx, timer.clone());
    //timer = Instant::now();
    //runtime.block_on(advanced_hole_punching(conn.clone(), timer));
    
    
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1080.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Quickr demo",
        options,
        Box::new(move |_cc| Ok(Box::new(app)))
    )
}




impl eframe::App for MyApp {

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.connected{
            while let Ok(frame) = self.frame_rx.try_recv() {
                // Process the received frame
                println!("Received a frame of size: {}", frame.data.len());
                let image = ColorImage::from_rgb(
                    [frame.width as usize, frame.height as usize],
                    &frame.data,
                );
                match &mut self.texture {
                    // reuse the existing texture instead of allocating a new one every frame
                    Some(tex) => tex.set(image, egui::TextureOptions::LINEAR),
                    None => {
                        self.texture = Some(ctx.load_texture(
                            "remote_screen",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
            }
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        while let Ok(result) = self.status_rx.try_recv() {
            self.connecting = false;
            match result {
                Ok(()) => {
                    self.connected = true;
                    self.runtime.spawn(udp_main_process(self.frame_tx.clone(), self.devtype.clone(), self.conn.clone(), self.timer));
                }
                Err(message) => {
                    self.error = Some(message);
                }
            }
        }
        // keep repainting while a connection attempt is in progress
        if self.connecting {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }





    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        
        if !self.connected{
            ui.heading("Welcome"); 
            ui.label(format!("Your own code is: {}", self.own_code));
            ui.add_space(10.0);
            ui.label(format!("Enter the destination code to connect:"));
            ui.add(
                egui::TextEdit::singleline(&mut self.remote_addr)
                    .hint_text("Ex: BA1F0DAF:c06a"),
            );
            if self.connecting {
                ui.label("Connecting...");
            } 
            else if ui.button("Connect").clicked() {
                let address = turn_code_to_ip(self.remote_addr.trim());
                match address.parse::<SocketAddr>() {
                    Ok(remote) => {
                        self.connecting = true;
                        self.error = None;
                        let conn = self.conn.clone();
                        let status_tx = self.status_tx.clone();
                        self.runtime.spawn(async move {
                            let result = async {
                                conn.connect(remote).await.map_err(|e| e.to_string())?;
                                advanced_hole_punching(conn, Instant::now())
                                    .await
                                    .map_err(|e| e.to_string())
                            }
                            .await;
                            let _ = status_tx.send(result);
                        });
                    }
                    Err(e) => self.error = Some(format!("Invalid code: {e}")),
                }
            }
            if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::RED, err);
            }
        }
        else{
            ui.heading("Screen Sharing");
            match &self.texture {
                Some(tex) => {
                    ui.add(egui::Image::new(tex).shrink_to_fit());
                }
                None => {
                    if self.devtype == "send" {
                        ui.label("Sharing your screen :)");
                    } else {
                        ui.label("Waiting for video...");
                    }
                }
            }
        }
    }
}