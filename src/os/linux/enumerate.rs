use crate::{UsbCore, UsbDevice};
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
        Ok(SysFs::usb_devices()
            .map_err(|e| Error::new(ErrorKind::Other, e))?
            .try_into()?)
    }

    fn add_device(&mut self, bus: u8, dev: u8) -> Result<(), std::io::Error> {
        // Try open read/write if fail try read
        let core = UsbCore::from_bus_device(bus, dev);
        let mut core = match core {
            Ok(core) => core,
            Err(_) => UsbCore::from_bus_device_read_only(bus, dev)?,
        };

        if let Some(dev) = core.take_descriptors() {
            let bus_address = format!("{}-{}", dev.bus_num, dev.dev_num);
            self.devices.insert(bus_address, dev);
        }
        Ok(())
    }

    pub fn devices(&self) -> &HashMap<String, UsbDevice> {
        &self.devices
    }

    pub fn get_device_from_bus(&self, bus: u8, address: u8) -> Option<&UsbDevice> {
        self.devices.get(&format!("{}-{}", bus, address))
    }
}
