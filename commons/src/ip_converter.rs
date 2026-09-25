use std::net::{IpAddr, Ipv4Addr, Ipv6Addr,SocketAddr};


pub fn turn_ip_to_code(ip: &IpAddr, port:&u16)-> String{
    let mut resp = String::new();
    let bytes: Vec<u8> = match ip {
        IpAddr::V4(ipv4) => ipv4.octets().to_vec(),
        IpAddr::V6(ipv6) => ipv6.octets().to_vec(),
    };
    for byte in bytes{
        let hex = format!("{:02X}", byte);
        resp.push_str(&hex);
    }
    let port_str = format!("{:X}", port); 
    resp.push(':');
    resp.push_str(&port_str);

    resp
}
pub fn turn_code_to_ip(code: &str) -> String {
    let information: Vec<&str> = code.split(':').collect();

    let hex_ip = information[0];
    let bytes: Vec<u8> = hex_ip
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let hex_pair = std::str::from_utf8(chunk).unwrap();
            u8::from_str_radix(hex_pair, 16).unwrap()
        })
        .collect();

    let ip = match bytes.len() {
        4 => IpAddr::V4(Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3])),
        16 => {
            let octets: [u8; 16] = bytes.try_into().expect("checked length is 16");
            IpAddr::V6(Ipv6Addr::from(octets))
        }
        other => panic!("unexpected address byte length: {other}"),
    };

    let port = u16::from_str_radix(information[1], 16).unwrap();

    SocketAddr::new(ip, port).to_string()
}