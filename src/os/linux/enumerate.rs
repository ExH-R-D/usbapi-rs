use crate::UsbDevice;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::io::{Error, ErrorKind};
use sysfs_serde::{SysFs, UsbDevices};

impl TryFrom<UsbDevices> for UsbEnumerate {
    type Error = std::io::Error;
    fn try_from(sysfs: UsbDevices) -> Result<Self, Self::Error> {
        let mut en = Self::default();
        for dev in sysfs.values() {
            let dev = UsbDevice::from_bytes(dev.descriptors.clone(), |mut d| {
                d.product = dev.product.clone();
                d.manufacturer = dev.manufacturer.clone();
                d.serial = dev.serial.clone();
                d.bus_num = dev.bus_num;
                d.dev_num = dev.dev_num;
            })?;
            en.devices
                .insert(format!("{}-{}", dev.bus_num, dev.dev_num), dev);
        }
        Ok(en)
    }
}

#[derive(Default)]
pub struct UsbEnumerate {
    pub devices: HashMap<String, UsbDevice>,
}

impl UsbEnumerate {
    pub fn from_sysfs() -> std::io::Result<Self> {
        SysFs::usb_devices()
            .map_err(|e| Error::new(ErrorKind::Other, e))?
            .try_into()
    }

    pub fn devices(&self) -> &HashMap<String, UsbDevice> {
        &self.devices
    }

    pub fn get_device_from_bus(&self, bus: u8, address: u8) -> Option<&UsbDevice> {
        self.devices.get(&format!("{}-{}", bus, address))
    }
}
