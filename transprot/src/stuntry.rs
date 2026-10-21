use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use stun::client::ClientBuilder;
use stun::message::*;
use stun::xoraddr::XorMappedAddress;

pub async fn discover_public_address(stun_server: &str, conn: Arc<UdpSocket>,) -> Result<XorMappedAddress, Box<dyn std::error::Error>> {
    // socket local: puede ser todos 0 pq igual despues se cambia
    match conn.local_addr() {
        Ok(addr) => eprintln!("[stun] local socket address before connect: {}", addr),
        Err(err) => eprintln!("[stun] unable to read local socket address before connect: {:?}", err),
    }

    eprintln!("[stun] connecting to STUN server: {}", stun_server);
    if let Err(err) = conn.connect(stun_server).await {
        eprintln!("[stun] connect failed for {}: {:?}", stun_server, err);
        return Err(Box::new(err));
    }
    eprintln!("[stun] connected to STUN server: {}", stun_server);

    eprintln!("[stun] creating STUN client");
    let mut client = match ClientBuilder::new().with_conn(conn.clone()).build() {
        Ok(client) => client,
        Err(err) => {
            eprintln!("[stun] STUN client build failed: {:?}", err);
            return Err(Box::new(err));
        }
    };

    // mensaje para enviar al stun server y que devuelva el stun ip y port
    let mut msg = Message::new();
    eprintln!("[stun] building BINDING_REQUEST");
    if let Err(err) = msg.build(&[Box::new(BINDING_REQUEST)]) {
        eprintln!("[stun] failed to build STUN message: {:?}", err);
        return Err(Box::new(err));
    }
    if let Err(err) = msg.new_transaction_id() {
        eprintln!("[stun] failed to create transaction ID: {:?}", err);
        return Err(Box::new(err));
    }

    let (handler_tx, mut handler_rx) = mpsc::unbounded_channel();
    eprintln!("[stun] sending STUN binding request");
    if let Err(err) = client.send(&msg, Some(Arc::new(handler_tx))).await {
        eprintln!("[stun] send failed: {:?}", err);
        return Err(Box::new(err));
    }

    eprintln!("[stun] waiting for STUN response");
    let event = match handler_rx.recv().await {
        Some(event) => event,
        None => {
            eprintln!("[stun] no response received from STUN server");
            return Err("no response received".into());
        }
    };

    let response = match event.event_body {
        Ok(response) => response,
        Err(err) => {
            eprintln!("[stun] event body parse failed: {:?}", err);
            return Err(Box::new(err));
        }
    };
    eprintln!("[stun] received STUN response, parsing XOR-MAPPED-ADDRESS");

    let mut xor_addr = XorMappedAddress::default();
    if let Err(err) = xor_addr.get_from(&response) {
        eprintln!("[stun] failed to parse XOR-MAPPED-ADDRESS from response: {:?}", err);
        return Err(Box::new(err));
    }

    eprintln!("[stun] closing STUN client");
    if let Err(err) = client.close().await {
        eprintln!("[stun] client close failed: {:?}", err);
        return Err(Box::new(err));
    }
    Ok(xor_addr)
}