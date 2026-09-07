use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use stun::client::ClientBuilder;
use stun::message::*;
use stun::xoraddr::XorMappedAddress;
//stun.rs
pub async fn discover_public_address(stun_server: &str, conn: Arc<UdpSocket>,) -> Result<XorMappedAddress, Box<dyn std::error::Error>> {
    // socket local: puede ser todos 0 pq igual despues se cambia
    println!("Local address: {}", &conn.local_addr()?);
    conn.connect(stun_server).await?; 

    let mut client = ClientBuilder::new().with_conn(conn.clone()).build()?;

    // mensaje para enviar al stun server y que devuelva el stun ip y port
    let mut msg = Message::new();
    msg.build(&[Box::new(BINDING_REQUEST)])?;
    msg.new_transaction_id()?;
    let (handler_tx, mut handler_rx) = mpsc::unbounded_channel();
    client.send(&msg, Some(Arc::new(handler_tx))).await?;

    let event = handler_rx.recv().await.ok_or("no response received")?;
    let response = event.event_body?;

    let mut xor_addr = XorMappedAddress::default();
    xor_addr.get_from(&response)?;
    client.close().await?;
    Ok(xor_addr)
}