use crate::descriptors::interface::Interface;
#[cfg(feature = "serde")]
use serde::Serialize;
use std::fmt;
use std::slice::Iter;
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug)]
pub struct Configuration {
    length: u8,
    kind: u8,
    total_length: u16,
    num_interfaces: u8,
    configuration_value: u8,
    iconfiguration: u8,
    bmattributes: u8,
    max_power: u8,
    pub interfaces: Vec<Interface>,
}

impl fmt::Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d += &format!("bDescriptorType: {}\n", self.kind);
        d += &format!("bTotalLength: {}\n", self.total_length);
        d += &format!("bNumInterfaces: {}\n", self.num_interfaces);
        d += &format!("bConfigurationValue: {}\n", self.configuration_value);
        d += &format!("iConfiguration: {}\n", self.iconfiguration);
        d += &format!("bmAttributes: 0x{:02x}\n", self.bmattributes);
        d += &format!("bMaxPower: {}\n", self.max_power);
        for iface in &self.interfaces {
            d += &format!("{}", iface);
        }
        write!(f, "{}", d)
    }
}

impl Configuration {
    pub fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(Configuration {
            length: *iter.next()?,
            kind: *iter.next()?,
            total_length: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            num_interfaces: *iter.next()?,
            configuration_value: *iter.next()?,
            iconfiguration: *iter.next()?,
            bmattributes: *iter.next()?,
            max_power: *iter.next()?,
            interfaces: vec![],
        })
    }
}
