fn main() {
    let n: u16 = 256;
    // 1 in little endian is; in big endian it is [0, 1]
    let bytes = n.to_be_bytes();
    println!("{:08b}",n)
}

