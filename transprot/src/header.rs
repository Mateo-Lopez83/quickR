use bytes::{Bytes, BytesMut, Buf, BufMut};
//use zerocopy::{self, ConvertError::Size};
use crate::PacketError;

pub const HEADERSIZE:usize = 17;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
    pub enum ChannelType {
        Video = 1,
        Audio = 2,
        Control = 3,
        Sync = 4,
    }
    impl TryFrom<u8> for ChannelType {
    type Error = PacketError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ChannelType::Video),
            2 => Ok(ChannelType::Audio),
            3 => Ok(ChannelType::Control),
            4 => Ok(ChannelType::Sync),
            _ => Err(PacketError::InvalidChannel(value)),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
    pub enum FragmentType {
        Start = 1,
        Middle = 2,
        End = 3,
        Unfragmented = 4
    }
    impl TryFrom<u8> for FragmentType {
    type Error = PacketError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(FragmentType::Start),
            2 => Ok(FragmentType::Middle),
            3 => Ok(FragmentType::End),
            4 => Ok(FragmentType::Unfragmented),
            _ => Err(PacketError::InvalidFragmenttype(value)),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
    pub struct Header {
        pub fragment: FragmentType,
        pub version: u8,        // 2 bits: version field
        pub padding: bool,       // 1 bit: define si tiene padding al final del paquete      
        pub channel: ChannelType,          //2 bits: define si es un paquete de video, audio o control
        pub frametype: bool,         // 1 bit: si es video, define si es keyframe o deltaframe
        pub sequence_number: u16,   //16 bits
        pub size: u16,             //cantidad de bytes del payload
        pub timestamp: u32,        //32 bits
        pub ssrc: u32,            //32 bits
        
    }


    impl Header{
        pub fn new(
                fragment: FragmentType,
                version:u8,
                padding: bool,      
                channel: ChannelType,          
                frametype: bool,         
                sequence_number: u16,  
                size: u16,          
                timestamp: u32,       
                ssrc: u32)->Header
        {
            Header { fragment, version, padding, channel, frametype, sequence_number, size, timestamp, ssrc }
        }

       
        
        pub fn serialize (&self)-> BytesMut{
            //función para transformar un struct de Header en una cadena de bytes del struct Bytes
            let mut buf = BytesMut::with_capacity(HEADERSIZE);
            buf.put_u8(self.fragment as u8);
            buf.put_u8(self.version);
            buf.put_u8(self.padding as u8);
            buf.put_u8(self.channel as u8);
            buf.put_u8(self.frametype as u8);

            buf.put_u16(self.sequence_number);
            buf.put_u16(self.size);

            buf.put_u32(self.timestamp);
            buf.put_u32(self.ssrc);

            buf
        }

        pub fn unserialize(buffer: &mut Bytes)->Result<Header,PacketError>{
            //inverso de serialize, transforma una cadena de bytes en un paquete
            
            if buffer.remaining() < HEADERSIZE{
                Err(PacketError::WrongBufferSize)
            }
            else {
                let fragment = FragmentType::try_from(buffer.get_u8())?;
                let version = buffer.get_u8();
                let padding = buffer.get_u8() != 0;
                //proceso para castear como el enum
                let channel_byte = buffer.get_u8();
                let channel = ChannelType::try_from(channel_byte)?;

                let frametype = buffer.get_u8() != 0;

                let sequence_number = buffer.get_u16();
                let size = buffer.get_u16();

                let timestamp = buffer.get_u32();
                let ssrc = buffer.get_u32();



                Ok(Header::new(fragment, version, padding, channel, frametype, sequence_number, size, timestamp, ssrc))
            }
        }
    }

    // impl ControlHeader{
    //     fn marshallize(&self)->Vec<u8>{
    //         let b0:u8;
    //         Vec::new()
    //     }
    // }

 // pub fn marshallize(&self)-> Vec<u8>{
        //     let mut full_header: Vec<u8> = vec![self.version];
        //     full_header.push(self.padding as u8);
        //     full_header.push(self.frametype as u8);

        //     //sequence
        //     for seq_byte in self.sequence_number.to_be_bytes(){
        //         full_header.push(seq_byte);
        //     }
        //     //size
        //     for seq_byte in self.size.to_be_bytes(){
        //         full_header.push(seq_byte);
        //     }
        //     //timestamp
        //     for seq_byte in self.timestamp.to_be_bytes(){
        //         full_header.push(seq_byte);
        //     }
        //     //ssrc
        //     for seq_byte in self.ssrc.to_be_bytes(){
        //         full_header.push(seq_byte);
        //     }
            
        //     full_header
        // }