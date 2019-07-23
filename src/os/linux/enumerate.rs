use std::io;
use std::io::BufReader;
use std::fs::{self,DirEntry, File};
use std::path::Path as Path;
use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::descriptors::device::Device;
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
    fn new(bus: u8, address: u8, device: Device) -> Self {
        UsbDevice {
            bus: bus,
            address: address,
            device: device
        }
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
        let file = match file {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{}", e);
                return ;
            }
        };

        let mut reader = BufReader::new(&file);
        let mut descs = Descriptor::from_buf_reader(&mut reader);
        let mut device: Option<UsbDevice> = None;
        if let Some(kind) = descs.next() {
            if let DescriptorType::Device(dev) = kind {
                device = Some(UsbDevice::new(bus, address, dev));
            } else {
                panic!("Could not enumerate device");
            }
        }
        let mut device = device.unwrap();
        for kind in descs {
            match kind {
                DescriptorType::Configuration(conf) => {
                    device.device.configurations.push(conf);
                }
                DescriptorType::Interface(iface) => {
                    self.add_interface(&mut device, iface);
                }
                DescriptorType::Endpoint(endpoint) => {
                    self.add_endpoint(&mut device, endpoint);
                }
                _ => {
                    //self.add_unknown(&mut device, &mut desc.iter());
                }
            };
        }
        let bus_address = format!("{}-{}", device.bus, device.address).to_string();
        self.devices.insert(bus_address, device);
    }

    fn add_unknown(&self, usb: &mut UsbDevice, desc: &mut Vec<u8>) {
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
        return &self.devices;
    }

    pub fn get_device_from_bus(&self, bus: u8, address: u8) -> Option<&UsbDevice> {
        let bus_address = format!("{}-{}", bus, address).to_string();
        self.devices.get(&bus_address)
    }
}

