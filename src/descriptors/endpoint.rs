#[cfg(feature = "serde")]
use serde::Serialize;
use std::fmt;
use std::slice::Iter;
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug)]
pub struct Endpoint {
    pub length: u8,
    pub kind: u8,
    pub endpoint_address: u8,
    pub bm_attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d += &format!("bDescriptorType: {}\n", self.kind);
        d += &format!("bEndpointAddress: 0x{:02X}\n", self.endpoint_address);
        d += &format!("bmAttributes: {}\n", self.bm_attributes);
        d += &format!("wMaxPacketSize: {}\n", self.max_packet_size);
        d += &format!("bInterval: {}\n", self.interval);
        write!(f, "{}", d)
    }
}

impl Endpoint {
    pub fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(Endpoint {
            length: *iter.next()?,
            kind: *iter.next()?,
            endpoint_address: *iter.next()?,
            bm_attributes: *iter.next()?,
            max_packet_size: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            interval: *iter.next()?,
        })
    }
}
