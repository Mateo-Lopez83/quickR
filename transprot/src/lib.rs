mod header;
mod stun;
pub mod udp;
use rand::RngExt;
pub mod error;
pub use error::PacketError;


#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};

use crate::header::{ChannelType, Header as head, FragmentType};
    use crate::{PacketError, RngExt};

    #[test]
    fn test_marsh_and_unmarsh() {
        let mut rng = rand::rng(); 
        let sequence: u16 = rng.random(); 
        let size: u16 = rng.random(); 
        let timestamp: u32 = rng.random(); 
        let ssrc: u32 = rng.random(); 
        let head_example = head::new(FragmentType::Start,2,
                                             false, 
                                             ChannelType::Video, 
                                             false, 
                                             sequence, 
                                             size, 
                                             timestamp, 
                                             ssrc);

        let mut serialized = head::serialize(&head_example).freeze();
        
        let deserialized = head::unserialize(&mut serialized).expect("Something went wrong");

        assert_eq!(head_example,deserialized);
    }
    #[test]
    fn check_wrong_channel(){
        let mut wrong_serial:BytesMut = BytesMut::new();
        let mut rng = rand::rng(); 
        wrong_serial.put_u8(1);
        wrong_serial.put_u8(2);
        wrong_serial.put_u8(0);
        //can be numbers 1-3, nothing else
        wrong_serial.put_u8(4);
        wrong_serial.put_u8(0);
        //seq number
        wrong_serial.put_u16(rng.random());
        //size
        wrong_serial.put_u16(rng.random());
        //timestamp
        wrong_serial.put_u32(rng.random());
        //ssrc
        wrong_serial.put_u32(rng.random());
        let mut bwrong_serial= wrong_serial.freeze();
        let deserialized = head::unserialize(&mut bwrong_serial);
        assert_eq!(deserialized, Err(PacketError::InvalidChannel(4)));
    }

    #[test]
    fn check_wrong_header_size(){
        let mut wrong_serial:BytesMut = BytesMut::new();
        let mut rng = rand::rng(); 
        wrong_serial.put_u8(2);
        wrong_serial.put_u8(2);
        wrong_serial.put_u8(0);
        wrong_serial.put_u8(1);
        wrong_serial.put_u8(0);
        //seq number
        wrong_serial.put_u16(rng.random());
        //size
        wrong_serial.put_u16(rng.random());
        //timestamp
        //ssrc
        wrong_serial.put_u32(rng.random());
        let mut bwrong_serial= wrong_serial.freeze();
        let deserialized = head::unserialize(&mut bwrong_serial);
        assert_eq!(deserialized, Err(PacketError::WrongBufferSize));
    }
    #[test]
    fn check_wrong_fragmentype(){
        let mut wrong_serial:BytesMut = BytesMut::new();
        let mut rng = rand::rng(); 
        //can be numbers 1-4, nothing else
        wrong_serial.put_u8(0);
        wrong_serial.put_u8(2);
        wrong_serial.put_u8(0);
        wrong_serial.put_u8(2);
        wrong_serial.put_u8(0);
        //seq number
        wrong_serial.put_u16(rng.random());
        //size
        wrong_serial.put_u16(rng.random());
        //timestamp
        wrong_serial.put_u32(rng.random());
        //ssrc
        wrong_serial.put_u32(rng.random());
        let mut bwrong_serial= wrong_serial.freeze();
        let deserialized = head::unserialize(&mut bwrong_serial);
        assert_eq!(deserialized, Err(PacketError::InvalidFragmenttype(0)));
    }

}