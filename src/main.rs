extern crate nix;
use nix::sys::ioctl;
use std::slice::Iter;
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
    pub configurations: Vec<ConfigurationDescriptor>
}

struct ConfigurationDescriptor {
    length: u8,
    descriptor_type: u8,
    total_length: u16,
    num_interfaces: u8,
    configuration_value: u8,
    iconfiguration: u8,
    bmattributes: u8,
    max_power: u8,
}

impl fmt::Display for DeviceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d+=&format!("bDescriptorType: {}\n", self.descriptor_type);
        d+=&format!("bcdUsb: 0x{:04x}\n", self.bcd_usb);
        d+=&format!("bDeviceClass: {}\n", self.device_class);
        d+=&format!("bDeviceSubClass: {}\n", self.device_sub_class);
        d+=&format!("bDeviceProtocol: {}\n", self.device_protocol);
        d+=&format!("bMaxPacketSize: {}\n", self.max_packet_size0);
        d+=&format!("idVendor: 0x{:04x}\n", self.id_vendor);
        d+=&format!("idProduct: 0x{:04x}\n", self.id_product);
        d+=&format!("bcdDevice: 0x{:04x}\n", self.bcd_device);
        d+=&format!("iManufacturer: {}\n", self.imanufacturer);
        d+=&format!("iProduct: {}\n", self.iproduct);
        d+=&format!("iSerialNumber: {}\n", self.iserial_number);
        d+=&format!("bNumConfigurations: {}\n", self.num_configurations);
        for conf in &self.configurations {
            d+=&format!("{}", conf);
        }
        write!(f, "{}", d)
    }
}

impl fmt::Display for ConfigurationDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d+=&format!("bDescriptorType: {}\n", self.descriptor_type);
        d+=&format!("bTotalLength: {}\n", self.total_length);
        d+=&format!("bNumInterfaces: {}\n", self.num_interfaces);
        d+=&format!("bConfigurationValue: {}\n", self.configuration_value);
        d+=&format!("iConfiguration: {}\n", self.iconfiguration);
        d+=&format!("bmAttributes: 0x{:02x}\n", self.bmattributes);
        d+=&format!("bMaxPower: {}\n", self.max_power);
        write!(f, "{}", d)
    }
}

impl DeviceDescriptor {
    fn new(iter: &mut Iter<u8>) -> Self {
        DeviceDescriptor {
            length: (iter.len() + 2) as u8,
            descriptor_type: 1,
            bcd_usb: *iter.next().unwrap_or(&0) as u16 | (*iter.next().unwrap_or(&0) as u16) << 8,
            device_class: *iter.next().unwrap_or(&0),
            device_sub_class: *iter.next().unwrap_or(&0),
            device_protocol: *iter.next().unwrap_or(&0),
            max_packet_size0: *iter.next().unwrap_or(&0),
            id_vendor: *iter.next().unwrap_or(&0) as u16 | (*iter.next().unwrap_or(&0) as u16) << 8,
            id_product: (*iter.next().unwrap_or(&0) as u16) | (*iter.next().unwrap_or(&0) as u16) << 8,
            bcd_device: *iter.next().unwrap_or(&0) as u16 | (*iter.next().unwrap_or(&0) as u16) << 8,
            imanufacturer: *iter.next().unwrap_or(&0),
            iproduct: *iter.next().unwrap_or(&0),
            iserial_number: *iter.next().unwrap_or(&0),
            num_configurations: *iter.next().unwrap_or(&0),
            configurations: vec![]
        }
    }
}

impl ConfigurationDescriptor {
    fn new(iter: &mut Iter<u8>) -> Self {
        ConfigurationDescriptor {
            length: iter.len() as u8,
            descriptor_type: 2,
            total_length: *iter.next().unwrap_or(&0) as u16 | (*iter.next().unwrap_or(&0) as u16) << 8,
            num_interfaces: *iter.next().unwrap_or(&0),
            configuration_value: *iter.next().unwrap_or(&0),
            iconfiguration: *iter.next().unwrap_or(&0),
            bmattributes: *iter.next().unwrap_or(&0),
            max_power: *iter.next().unwrap_or(&0)
        }
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
        let file = File::open(file.path());
        let mut file = match file {
            Ok(file) => file,
            Err(e) => {
                eprintln!("{}", e);
                return ;
            }
        };

        // FIME This madness can seriosly be done better
        // Need to read some more vec/iterator tutorials.
        let mut data = vec![];
        file.read_to_end(&mut data).expect("Failed to read");
        let mut iter = data.iter();
        let dlength = *iter.next().unwrap_or(&0) as usize;
        let typ = *iter.next().unwrap_or(&0);
        // This feels awkward should use take(dlength) or similar here...
        let mut iter_desc = data[2..dlength].iter();
        let mut desc = DeviceDescriptor::new(&mut iter_desc);
        let mut confs = desc.num_configurations;
        let mut start = dlength;
        while confs > 0 {
            confs-= 1;
            let mut iter = data[start..start+2].iter();
            start+=2;
            let dlength = *iter.next().unwrap_or(&0) as usize;
            let typ = *iter.next().unwrap_or(&0);
            let mut iter_desc = data[start..start+dlength].iter();
            start+=dlength;
            let conf = ConfigurationDescriptor::new(&mut iter_desc);
            desc.configurations.push(conf);
        }

        println!("{}:{}\n{}", bus, address, desc);
    }

}

fn main() {
    let mut usb = LinuxUsbDevices::new();
    usb.enumerate(Path::new("/dev/bus/usb/"));
}
