extern crate nix;
use nix::sys::ioctl;
use std::io;
use std::fs::{self,DirEntry, File};
use std::path::Path as Path;
use std::io::prelude::*;
use std::fmt;

struct LinuxUsbDevices {
}

struct DeviceDescriptor {
    length: u8,
    descriptor_type: u8,
    bcd_usb: u16,
    device_class: u8,
    device_sub_class: u8,
    device_protocol: u8,
    max_packet_size0: u8,
    id_vendor: u16,
    id_product: u16,
    bcd_device: u16,
    imanufacturer: u8,
    iproduct: u8,
    iserial_number: u8,
    num_configurations: u8,
    configurations: Vec<u8>
}

struct ConfigurationDescriptor {
    length: u8,
    descriptor_type: u8,
    total_length: u16,
    num_interfaces: u8,
    configuration_value: u8,
    bmattributes: u8,
    max_power: u8,
    extra: Vec<u8>
}

impl fmt::Display for DeviceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("length: {}\n", self.length);
        d+=&format!("descriptor_type: {}\n", self.descriptor_type);
        d+=&format!("bcd_usb: {:04X}\n", self.bcd_usb);
        d+=&format!("device_class: {}\n", self.device_class);
        d+=&format!("device_sub_class: {}\n", self.device_sub_class);
        d+=&format!("device_protocol: {}\n", self.device_protocol);
        d+=&format!("max_packet_size: {}\n", self.max_packet_size0);
        d+=&format!("id_vendor: 0x{:04X}\n", self.id_vendor);
        d+=&format!("id_product: 0x{:04X}\n", self.id_product);
        d+=&format!("bcd_device: 0x{:04X}\n", self.bcd_device);
        d+=&format!("imanufacturer: {}\n", self.imanufacturer);
        d+=&format!("iproduct: {}\n", self.iproduct);
        d+=&format!("iserial_number: {}\n", self.iserial_number);
        d+=&format!("num_configurations: {}\n", self.num_configurations);
        d+=&format!("configurations: {:02X?}\n", self.configurations);

        write!(f, "{}", d)
    }
}

impl DeviceDescriptor {
    fn new(data: Vec<u8>) -> Self {
        let mut desc = DeviceDescriptor {
            length: 0,
            descriptor_type: 0,
            bcd_usb: 0,
            device_class: 0,
            device_sub_class: 0,
            device_protocol: 0,
            max_packet_size0: 0,
            id_vendor: 0,
            id_product: 0,
            bcd_device: 0,
            imanufacturer: 0,
            iproduct: 0,
            iserial_number: 0,
            num_configurations: 0,
            configurations: vec![]
        };

        let mut iter = data.iter();
        desc.length = *iter.next().unwrap_or(&0);
        desc.descriptor_type = *iter.next().unwrap_or(&0);
        desc.bcd_usb = *iter.next().unwrap_or(&0) as u16;
        desc.bcd_usb|= (*iter.next().unwrap_or(&0) as u16) << 8;
        desc.device_class = *iter.next().unwrap_or(&0);
        desc.device_sub_class = *iter.next().unwrap_or(&0);
        desc.device_protocol = *iter.next().unwrap_or(&0);
        desc.max_packet_size0 = *iter.next().unwrap_or(&0);
        desc.id_vendor = *iter.next().unwrap_or(&0) as u16;
        desc.id_vendor |= (*iter.next().unwrap_or(&0) as u16) << 8;
        desc.id_product = (*iter.next().unwrap_or(&0) as u16);
        desc.id_product |= (*iter.next().unwrap_or(&0) as u16) << 8;
        desc.bcd_device = *iter.next().unwrap_or(&0) as u16;
        desc.bcd_device|= (*iter.next().unwrap_or(&0) as u16) << 8;
        desc.imanufacturer = *iter.next().unwrap_or(&0);
        desc.iproduct = *iter.next().unwrap_or(&0);
        desc.iserial_number = *iter.next().unwrap_or(&0);
        desc.num_configurations = *iter.next().unwrap_or(&0);
        for d in iter {
            desc.configurations.push(*d);
        }
        desc
    }
}

impl LinuxUsbDevices {
    fn new() -> Self {
        LinuxUsbDevices{}
    }
    fn enumerate(&mut self, dir: &Path) -> io::Result<()> {
        for entry in fs::read_dir(dir).expect("Can't acces usbpath?") {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => { eprintln!("{}", e); continue; },
            };
            let path = entry.path();
            if path.is_dir() {
                self.enumerate(&path);
            } else {
                let bus: i16 = path.parent().unwrap().file_name().unwrap().to_string_lossy().parse::<i16>().unwrap_or(0);
                let address: i16 = path.file_name().unwrap().to_string_lossy().parse::<i16>().unwrap_or(0);
                self.device(&entry, bus, address);
            }
        }
        Ok(())
    }

    pub fn device(&mut self, file: &DirEntry, bus: i16, address: i16) {
        let mut file = File::open(file.path());
        let mut file = match file {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{}", e);
                return ;
            }
        };

        let mut data = vec![];
        file.read_to_end(&mut data).expect("Failed to read");
        let desc = DeviceDescriptor::new(data);
        println!("{}:{}\n{}", bus, address, desc);
    }

}

fn main() {
    let mut usb = LinuxUsbDevices::new();
    usb.enumerate(Path::new("/dev/bus/usb/"));
}
