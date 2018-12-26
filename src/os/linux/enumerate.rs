use std::io;
use std::fs::{self,DirEntry, File};
use std::path::Path as Path;
use std::io::prelude::*;
use std::slice::Iter;
use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::descriptors::device::Device;
use crate::descriptors::configuration::Configuration;
use crate::descriptors::interface::Interface;
use crate::descriptors::endpoint::Endpoint;
use crate::descriptors::descriptor::{Descriptor, DescriptorType};

#[derive(Serialize, Deserialize)]
pub struct UsbDevice {
    pub bus: u8,
    pub address: u8,
    pub device: Device
}

impl fmt::Display for UsbDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}\n{}", self.bus, self.address, self.device)
    }
}

impl UsbDevice {
    fn new(bus: u8, address: u8, iter: &mut Iter<u8>) -> Option<Self> {
        Some(UsbDevice {
            bus: bus,
            address: address,
            device: Device::new(iter)?
        })
    }
}

pub struct UsbEnumerate {
    pub devices: HashMap<String, UsbDevice>,
}

impl UsbEnumerate {
    pub fn new() -> Self {
        UsbEnumerate { devices: HashMap::new()}
    }

    pub fn enumerate(&mut self) -> io::Result<()> {
        self.read_dir(Path::new("/dev/bus/usb/"))
    }

    fn read_dir(&mut self, dir: &Path) -> io::Result<()> {
        for entry in fs::read_dir(dir).expect("Can't acces usbpath?") {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => { eprintln!("{}", e); continue; },
            };
            let path = entry.path();
            if path.is_dir() {
                self.read_dir(&path)?;
            } else {
                let bus: u8 = path.parent().unwrap().file_name().unwrap().to_string_lossy().parse::<u8>().expect("Something is broken could not parse bus as u8");
                let address: u8 = path.file_name().unwrap().to_string_lossy().parse::<u8>().expect("Something is smoking could not parse address from dirname {}");
                self.add_device(&entry, bus, address);
            }
        }
        Ok(())
    }

    fn add_device(&mut self, file: &DirEntry, bus: u8, address: u8) {
        let file = File::open(file.path());
        let mut file = match file {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{}", e);
                return ;
            }
        };

        let mut desc = Descriptor::new(Vec::new());
        file.read_to_end(&mut desc.descriptor).expect("Could not read Descriptor");
        let device = desc.next().unwrap();
        let mut device = UsbDevice::new(bus, address, &mut device.iter()).expect("Could not add DeviceDescriptor");
        for current in desc {
            // still unhappy with my implemention could probably be done better...
            let kind = current[1];// as DescriptorType;
            match DescriptorType::from(kind){
                DescriptorType::Configuration => {
                    self.add_configuration(&mut device, &mut current.iter())
                },
                DescriptorType::Interface => {
                    self.add_interface(&mut device, &mut current.iter())
                },
                DescriptorType::Endpoint => {
                    self.add_endpoint(&mut device, &mut current.iter())
                }
                _ => {
                    println!("{}:{} FIXME DescriptorType {} {:02X?}", device.bus, device.address, kind, current);
                    continue;
                }
            };
        }
        let bus_address = format!("{}-{}", device.bus, device.address).to_string();
        self.devices.insert(bus_address, device);
    }

    fn add_configuration(&self, usb: &mut UsbDevice, iter_desc: &mut Iter<u8>) {
        match Configuration::new(iter_desc) {
            Some(conf) => usb.device.configurations.push(conf),
            None => eprintln!("Could not parse Configuration descriptor {:02X?} for {}:{}", iter_desc, usb.bus, usb.address)
        };
    }

    fn add_interface(&self, usb: &mut UsbDevice, iter_desc: &mut Iter<u8>) {
        let configuration = usb.device.configurations.last_mut().unwrap();
        match Interface::new(iter_desc) {
            Some(iface) => configuration.interfaces.push(iface),
            None => eprintln!("Could not parse Interface descriptor {:02X?} for {}:{}", iter_desc, usb.bus, usb.address)
        };
    }

    fn add_endpoint(&self, usb: &mut UsbDevice, iter_desc: &mut Iter<u8>) {
        let configuration = usb.device.configurations.last_mut().unwrap();
        let endpoints = &mut configuration.interfaces.last_mut().unwrap().endpoints;
        match Endpoint::new(iter_desc) {
            Some(endpoint) => {
                endpoints.push(endpoint);
            },
            None => eprintln!("Could not parse Endpoint descriptor {:02X?} for {}:{}", iter_desc, usb.bus, usb.address)
        };
    }

    pub fn devices(&self) -> &HashMap<String, UsbDevice> {
        return &self.devices;
    }

    pub fn get_device_from_bus(&self, bus: u8, address: u8) -> Option<&UsbDevice> {
        let bus_address = format!("{}-{}", bus, address).to_string();
        self.devices.get(&bus_address)
    }
}

