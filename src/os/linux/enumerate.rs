use crate::{UsbCore, UsbDevice};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use sysfs_bus::SysFs;

#[derive(Default)]
pub struct UsbEnumerate {
    pub devices: HashMap<String, UsbDevice>,
}

impl UsbEnumerate {
    #[deprecated(since = "0.1.0", note = "please use `from_sysfs` instead")]
    pub fn from_usbfs() -> io::Result<Self> {
        let mut e = Self::default();
        e.read_dir(Path::new("/dev/bus/usb/"))?;
        Ok(e)
    }

    pub fn from_sysfs() -> io::Result<Self> {
        let mut e = Self::default();
        let devices = SysFs::usb_devices()?;
        for dev in devices.values() {
            let dev = UsbDevice::from_bytes(dev.descriptors.clone(), |mut d| {
                println!("{}", dev.product);
                d.product = dev.product.clone();
                d.manufacturer = dev.manufacturer.clone();
                d.serial = dev.serial.clone();
                d.bus_num = dev.bus_num.unwrap();
                d.dev_num = dev.dev_num.unwrap();
            })?;
            e.devices
                .insert(format!("{}-{}", dev.bus_num, dev.dev_num), dev);
        }

        Ok(e)
    }

    #[deprecated(since = "0.1.0", note = "please use `from_sysfs` instead")]
    fn read_dir(&mut self, dir: &Path) -> io::Result<()> {
        for entry in fs::read_dir(dir).expect("Can't access usbpath?") {
            if let Err(e) = entry {
                log::error!("Could not read {:?} entry cause {}", dir, e);
                continue;
            }

            let path = entry.and_then(|e| Ok(e.path()))?;
            if path.is_dir() {
                self.read_dir(&path)?;
            } else {
                let bus: u8 = path
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .parse::<u8>()
                    .expect("Something is broken could not parse bus as u8");
                let address: u8 = path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .parse::<u8>()
                    .expect("Something is smoking could not parse address from dirname {}");
                self.add_device(bus, address).unwrap_or_else(|e| {
                    log::error!(
                        "Could not read descriptors on USB: {}-{} cause {}",
                        bus,
                        address,
                        e
                    )
                });
            }
        }
        Ok(())
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
