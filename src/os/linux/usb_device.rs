use crate::descriptors::descriptor::{Descriptor, DescriptorType};
use crate::descriptors::device::Device;
use crate::UsbCore;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::prelude::*;
#[derive(Debug, Serialize, Deserialize)]
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

    pub(crate) fn from_usb_raw(usb: &mut UsbCore) -> Option<Self> {
        let mut bytes = Vec::new();
        if usb.handle().read_to_end(&mut bytes).is_err() {
            log::error!(
                "Could not read descriptors on device {}-{}",
                usb.bus_dev.0,
                usb.bus_dev.1
            );
            return None;
        }

        Self::from_bytes(bytes, |mut d| {
            d.bus_num = usb.bus_dev.0;
            d.dev_num = usb.bus_dev.1;
            d.manufacturer = usb.get_descriptor_string(d.device.imanufacturer.clone());
            d.product = usb.get_descriptor_string(d.device.iproduct.clone());
            d.serial = usb.get_descriptor_string(d.device.iserial.clone());
        })
    }

    pub(crate) fn from_bytes<F>(vec: Vec<u8>, mut f: F) -> Option<Self>
    where
        F: FnMut(&mut Self),
    {
        let descs = if let Ok(mut descs) = Descriptor::from_bytes(vec.bytes()) {
            if let DescriptorType::Device(dev) = descs
                .next()
                .unwrap_or_else(|| DescriptorType::Unknown(vec![]))
            {
                let mut device: UsbDevice =
                    UsbDevice::new(0, 0, dev, String::new(), String::new(), String::new());
                f(&mut device);
                Some((descs, device))
            } else {
                log::error!("Descriptor read failed");
                None
            }
        } else {
            return None;
        };

        let (descs, mut device) = descs?;
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
        Some(device)
    }
}
