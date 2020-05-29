use crate::descriptors::descriptor::{Descriptor, DescriptorType};
use crate::descriptors::device::Device;
use crate::descriptors::endpoint::Endpoint;
use crate::descriptors::interface::Interface;
use crate::UsbCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct UsbDevice {
    pub bus: u8,
    pub address: u8,
    pub manufacturer: String,
    pub product: String,
    pub device: Device,
}

impl fmt::Display for UsbDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}\n{}", self.bus, self.address, self.device)
    }
}

impl UsbDevice {
    fn new(bus: u8, dev: u8, device: Device, manufacturer: String, product: String) -> Self {
        UsbDevice {
            bus,
            address: dev,
            device,
            product,
            manufacturer,
        }
    }
}

#[derive(Default)]
pub struct UsbEnumerate {
    pub devices: HashMap<String, UsbDevice>,
}

impl UsbEnumerate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enumerate(&mut self) -> io::Result<()> {
        self.read_dir(Path::new("/dev/bus/usb/"))
    }

    fn read_dir(&mut self, dir: &Path) -> io::Result<()> {
        for entry in fs::read_dir(dir).expect("Can't access usbpath?") {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("{}", e);
                    continue;
                }
            };
            let path = entry.path();
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
                self.add_device(&entry, bus, address);
            }
        }
        Ok(())
    }

    fn add_device(&mut self, file: &DirEntry, bus: u8, dev: u8) {
        if let Some(mut descs) = Descriptor::from_file(&file.path()) {
            let mut device;
            if let Some(kind) = descs.next() {
                if let DescriptorType::Device(devicet) = kind {
                    let usb = UsbCore::from_bus_device(bus, dev);
                    match usb {
                        Ok(mut usb) => {
                            let vendor = usb.get_descriptor_string(devicet.imanufacturer.clone());
                            let product = usb.get_descriptor_string(devicet.iproduct.clone());
                            device = UsbDevice::new(bus, dev, devicet, vendor, product);
                        }
                        Err(_) => {
                            device = UsbDevice::new(bus, dev, devicet, "".into(), "".into());
                        }
                    }
                } else {
                    panic!("Could not enumerate device");
                }
            } else {
                panic!("{}-{} has no descriptors?", bus, dev);
            }
            for kind in descs {
                match kind {
                    DescriptorType::Configuration(conf) => {
                        device.device.configurations.push(conf);
                    }
                    DescriptorType::Interface(iface) => {
                        self.add_interface(&mut device, iface);
                    }
                    DescriptorType::String(_) => {}
                    DescriptorType::Endpoint(endpoint) => {
                        self.add_endpoint(&mut device, endpoint);
                    }
                    _ => {
                        //self.add_unknown(&mut device, &mut desc.iter());
                    }
                };
            }
            let bus_address = format!("{}-{}", device.bus, device.address);
            self.devices.insert(bus_address, device);
        }
    }

    fn add_interface(&self, usb: &mut UsbDevice, iface: Interface) {
        let configuration = usb.device.configurations.last_mut().unwrap();
        configuration.interfaces.push(iface);
    }

    fn add_endpoint(&self, usb: &mut UsbDevice, endpoint: Endpoint) {
        let configuration = usb.device.configurations.last_mut().unwrap();
        let endpoints = &mut configuration.interfaces.last_mut().unwrap().endpoints;
        endpoints.push(endpoint);
    }

    pub fn devices(&self) -> &HashMap<String, UsbDevice> {
        &self.devices
    }

    pub fn get_device_from_bus(&self, bus: u8, address: u8) -> Option<&UsbDevice> {
        self.devices.get(&format!("{}-{}", bus, address))
    }
}
