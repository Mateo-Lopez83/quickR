use std::env;
use transprot::udp;




#[tokio::main]
async fn main() {
    let devtype:String = env::args().nth(1).expect("you have to specify if this is a sender or receiver");
    match devtype.as_str() {
        "send"=>{
            let address = env::args().nth(2).expect("you have to specify the ip address of the receiver");
            let _ = udp::main_sending_process(address).await;
        },
        "receive"=>{
            let _ = udp::receiving_process().await;
        },
        othe =>{
            println!("No command associated with {}",othe);
        }

        
    }
}
