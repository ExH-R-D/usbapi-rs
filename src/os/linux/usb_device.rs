use crate::descriptors::descriptor::{Descriptor, DescriptorType};
use crate::descriptors::device::Device;
use crate::UsbCore;
#[cfg(feature = "serde")]
use serde::Serialize;
use std::fmt;
use std::io::prelude::*;
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug)]
pub struct UsbDevice {
    pub bus_num: u8,
    pub dev_num: u8,
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
    pub device: Device,
}

impl fmt::Display for UsbDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}\n{}", self.bus_num, self.dev_num, self.device)
    }
}

impl UsbDevice {
    pub(crate) fn new(
        bus_num: u8,
        dev_num: u8,
        device: Device,
        manufacturer: String,
        product: String,
        serial: String,
    ) -> Self {
        UsbDevice {
            bus_num,
            dev_num,
            device,
            product,
            manufacturer,
            serial,
        }
    }

    pub(crate) fn from_bytes<F>(
        vec: Vec<u8>,
        mut fill_descriptor_strings: F,
    ) -> Result<Self, std::io::Error>
    where
        F: FnMut(&mut Self),
    {
        use std::io::{Error, ErrorKind};
        let mut descs = Descriptor::from_bytes(vec.bytes())?;
        // The first descriptor should be the device
        // If not well then something is bad
        let device = match descs.next() {
            Some(dev) => match dev {
                DescriptorType::Device(d) => Ok(d),
                _ => Err(Error::new(
                    ErrorKind::Other,
                    format!(
                        "Wrong descriptor detected: {:?} expected DeviceDescriptor",
                        dev
                    ),
                )),
            },
            None => Err(Error::new(
                ErrorKind::Other,
                format!("No device descriptor found. {:?}", vec),
            )),
        }?;
        let mut device: UsbDevice =
            UsbDevice::new(0, 0, device, String::new(), String::new(), String::new());
        fill_descriptor_strings(&mut device);

        for kind in descs {
            match kind {
                DescriptorType::Configuration(conf) => {
                    device.device.configurations.push(conf);
                }
                DescriptorType::Interface(iface) => {
                    if let Some(c) = device.device.configurations.last_mut() {
                        c.interfaces.push(iface);
                    }
                }
                DescriptorType::String(_) => {}
                DescriptorType::Endpoint(endpoint) => {
                    if let Some(c) = device.device.configurations.last_mut() {
                        if let Some(i) = c.interfaces.last_mut() {
                            i.endpoints.push(endpoint);
                        }
                    }
                }
                _ => {
                    log::debug!("Unknown descriptor type: {:?}", kind);
                }
            };
        }
        Ok(device)
    }

    pub fn from_usbcore(usb: &mut UsbCore) -> Result<Self, std::io::Error> {
        let mut bytes = Vec::new();
        usb.handle().read_to_end(&mut bytes)?;
        Self::from_bytes(bytes, |mut d| {
            d.bus_num = usb.bus_dev.0;
            d.dev_num = usb.bus_dev.1;
            d.manufacturer = usb
                .get_descriptor_string(d.device.imanufacturer)
                .unwrap_or_else(|_| String::from(""));
            d.product = usb
                .get_descriptor_string(d.device.iproduct)
                .unwrap_or_else(|_| String::new());
            d.serial = usb
                .get_descriptor_string(d.device.iserial)
                .unwrap_or_else(|_| String::new());
        })
    }
}
