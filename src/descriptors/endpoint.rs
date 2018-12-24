use std::slice::Iter;
use std::fmt;
#[derive(Serialize, Deserialize)]
pub struct Endpoint {
    length: u8,
    kind: u8,
    endpoint_address: u8,
    bm_attributes: u8,
    max_packet_size: u16,
    interval: u8
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d+=&format!("bDescriptorType: {}\n", self.kind);
        d+=&format!("bEndpointAddress: 0x{:02X}\n", self.endpoint_address);
        d+=&format!("bmAttributes: {}\n", self.bm_attributes);
        d+=&format!("wMaxPacketSize: {}\n", self.max_packet_size);
        d+=&format!("bInterval: {}\n", self.interval);
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
            interval: *iter.next()?
        })
    }
}

