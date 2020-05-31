use crate::descriptors::configuration::Configuration;
use serde::{Deserialize, Serialize};
use serde_hex::{SerHex, StrictPfx};
use std::fmt;
use std::slice::Iter;

#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    pub length: u8,
    pub kind: u8,
    #[serde(with = "SerHex::<StrictPfx>")]
    pub bcd_usb: u16,
    pub device_class: u8,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size0: u8,
    #[serde(with = "SerHex::<StrictPfx>")]
    pub id_vendor: u16,
    #[serde(with = "SerHex::<StrictPfx>")]
    pub id_product: u16,
    #[serde(with = "SerHex::<StrictPfx>")]
    pub bcd_device: u16,
    pub imanufacturer: u8,
    pub iproduct: u8,
    pub iserial: u8,
    pub num_configurations: u8,
    pub configurations: Vec<Configuration>,
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
