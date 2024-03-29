use crate::descriptors::endpoint::Endpoint;
#[cfg(feature = "serde")]
use serde::Serialize;
use std::fmt;
use std::slice::Iter;
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug)]
pub struct Interface {
    pub length: u8,
    pub kind: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_sub_class: u8,
    pub interface_protocol: u8,
    pub iinterface: u8,
    pub endpoints: Vec<Endpoint>,
}

impl fmt::Display for Interface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d += &format!("bDescriptorType: {}\n", self.kind);
        d += &format!("bInterfaceNumber: {}\n", self.interface_number);
        d += &format!("bAlternateSetting: {}\n", self.alternate_setting);
        d += &format!("bNumEndpoints: {}\n", self.num_endpoints);
        d += &format!("bInterfaceClass: {}\n", self.interface_class);
        d += &format!("bInterfaceSubClass: {}\n", self.interface_sub_class);
        d += &format!("bInterfaceProtocol: {}\n", self.interface_protocol);
        d += &format!("bInterfaceNumber: {}\n", self.interface_number);
        d += &format!("iInterface: {}\n", self.iinterface);
        for endpoint in &self.endpoints {
            d += &format!("{}", endpoint);
        }
        write!(f, "{}", d)
    }
}

impl Interface {
    pub fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(Interface {
            length: *iter.next()?,
            kind: *iter.next()?,
            interface_number: *iter.next()?,
            alternate_setting: *iter.next()?,
            num_endpoints: *iter.next()?,
            interface_class: *iter.next()?,
            interface_sub_class: *iter.next()?,
            interface_protocol: *iter.next()?,
            iinterface: *iter.next()?,
            endpoints: vec![],
        })
    }
}
