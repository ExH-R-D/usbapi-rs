use crate::descriptors::configuration::Configuration;
#[cfg(feature = "serde")]
use serde::{Serialize, Serializer};
use std::fmt;
use std::slice::Iter;

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug)]
pub struct Device {
    pub length: u8,
    pub kind: u8,
    #[cfg_attr(feature = "serde", serde(serialize_with = "to_hex16"))]
    pub bcd_usb: u16,
    pub device_class: u8,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size0: u8,
    #[cfg_attr(feature = "serde", serde(serialize_with = "to_hex16"))]
    pub id_vendor: u16,
    #[cfg_attr(feature = "serde", serde(serialize_with = "to_hex16"))]
    pub id_product: u16,
    #[cfg_attr(feature = "serde", serde(serialize_with = "to_hex16"))]
    pub bcd_device: u16,
    pub imanufacturer: u8,
    pub iproduct: u8,
    pub iserial: u8,
    pub num_configurations: u8,
    pub configurations: Vec<Configuration>,
}

#[cfg(feature = "serde")]
fn to_hex16<S>(id_vendor: &u16, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("0x{:04X}", id_vendor))
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Device {
    pub fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(Device {
            length: *iter.next()?,
            kind: *iter.next()?,
            bcd_usb: *iter.next()? as u16 | (*iter.next().unwrap_or(&0) as u16) << 8,
            device_class: *iter.next()?,
            device_sub_class: *iter.next()?,
            device_protocol: *iter.next()?,
            max_packet_size0: *iter.next()?,
            id_vendor: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            id_product: (*iter.next()? as u16) | (*iter.next()? as u16) << 8,
            bcd_device: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            imanufacturer: *iter.next()?,
            iproduct: *iter.next()?,
            iserial: *iter.next()?,
            num_configurations: *iter.next()?,
            configurations: vec![],
        })
    }
}
