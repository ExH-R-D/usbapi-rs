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

enum DescriptorType {
    Device = 1,
    Configuration = 2,
    UTFString = 3,
    Interface = 4,
    Endpoint = 5,
    Class_Specific = 0x24,
    Hub = 0x29,
    SS_Endpoint_Companion = 0x30
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
    pub interfaces: Vec<InterfaceDescriptor>
}

struct InterfaceDescriptor {
    length: u8,
    descriptor_type: u8,
    interface_number: u8,
    alternate_setting: u8,
    num_endpoints: u8,
    interface_class: u8,
    interface_sub_class:u8,
    interface_protocol: u8,
    iinterface: u8,
    pub endpoints: Vec<EndpointDescriptor>
}

struct EndpointDescriptor {
    length: u8,
    descriptor_type: u8,
    endpoint_address: u8,
    bm_attributes: u8,
    max_packet_size: u16,
    interval: u8
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
        for iface in &self.interfaces {
            d+=&format!("{}", iface);
        }
        write!(f, "{}", d)
    }
}

impl fmt::Display for InterfaceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d+=&format!("bDescriptorType: {}\n", self.descriptor_type);
        d+=&format!("bInterfaceNumber: {}\n", self.interface_number);
        d+=&format!("bAlternateSetting: {}\n", self.alternate_setting);
        d+=&format!("bNumEndpoints: {}\n", self.num_endpoints);
        d+=&format!("bInterfaceClass: {}\n", self.interface_class);
        d+=&format!("bInterfaceSubClass: {}\n", self.interface_sub_class);
        d+=&format!("bInterfaceProtocol: {}\n", self.interface_protocol);
        d+=&format!("bInterfaceNumber: {}\n", self.interface_number);
        d+=&format!("iInterface: {}\n", self.iinterface);
        for endpoint in &self.endpoints {
            d+=&format!("{}", endpoint);
        }
        write!(f, "{}", d)
    }
}

impl fmt::Display for EndpointDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d+=&format!("bDescriptorType: {}\n", self.descriptor_type);
        d+=&format!("bEndpointAddress: 0x{:02X}\n", self.endpoint_address);
        d+=&format!("bmAttributes: {}\n", self.bm_attributes);
        d+=&format!("wMaxPacketSize: {}\n", self.max_packet_size);
        d+=&format!("bInterval: {}\n", self.interval);
        write!(f, "{}", d)
    }
}

impl DeviceDescriptor {
    fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(DeviceDescriptor {
            length: *iter.next()?,
            descriptor_type: *iter.next()?,
            bcd_usb: *iter.next()? as u16 | (*iter.next().unwrap_or(&0) as u16) << 8,
            device_class: *iter.next()?,
            device_sub_class: *iter.next()?,
            device_protocol: *iter.next()?,
            max_packet_size0: *iter.next()?,
            id_vendor: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            id_product: (*iter.next()? as u16) | (*iter.next()? as u16) << 8,
            bcd_device: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            imanufacturer: *iter.next()?,
            iproduct: *iter.next()?,
            iserial_number: *iter.next()?,
            num_configurations: *iter.next()?,
            configurations: vec![]
        })
    }
}

impl ConfigurationDescriptor {
    fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(ConfigurationDescriptor {
            length: *iter.next()?,
            descriptor_type: *iter.next()?,
            total_length: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            num_interfaces: *iter.next()?,
            configuration_value: *iter.next()?,
            iconfiguration: *iter.next()?,
            bmattributes: *iter.next()?,
            max_power: *iter.next()?,
            interfaces: vec![]
        })
    }
}

impl InterfaceDescriptor {
    fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(InterfaceDescriptor {
            length: *iter.next()?,
            descriptor_type: *iter.next()?,
            interface_number: *iter.next()?,
            alternate_setting: *iter.next()?,
            num_endpoints: *iter.next()?,
            interface_class: *iter.next()?,
            interface_sub_class: *iter.next()?,
            interface_protocol: *iter.next()?,
            iinterface: *iter.next()?,
            endpoints: vec![]
        })
    }
}

impl EndpointDescriptor {
    fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(EndpointDescriptor {
            length: *iter.next()?,
            descriptor_type: *iter.next()?,
            endpoint_address: *iter.next()?,
            bm_attributes: *iter.next()?,
            max_packet_size: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            interval: *iter.next()?
        })
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
        let mut iter_desc = data[..dlength].iter();
        let mut desc = DeviceDescriptor::new(&mut iter_desc).expect("Could not parse desc");
        let mut confs = desc.num_configurations;
        let mut data = &data[dlength..];
        while confs > 0 {
            confs-= 1;
            let mut iter = data[..2].iter();
            let dlength = *iter.next().unwrap_or(&0) as usize;
            let typ = *iter.next().unwrap_or(&0);
            let mut iter_desc = data[..dlength].iter();
            let mut conf = ConfigurationDescriptor::new(&mut iter_desc).expect("Could not parse config desciprtiion");
            data = &data[dlength..];
            let mut numifaces = conf.num_interfaces;
            while numifaces > 0 {
                let mut iter = data[..2].iter();
                let mut dlength = *iter.next().unwrap_or(&0) as usize;
                let typ = *iter.next().unwrap_or(&0);
                let mut iter_desc = data[..dlength].iter();
                // FIXME enum and redo this madness
                match typ {
                    4 => match InterfaceDescriptor::new(&mut iter_desc) {
                        Some(mut iface) => {
                            numifaces-=1;
                            let mut epoints = iface.num_endpoints;
                            while epoints > 0 {
                                data = &data[dlength..];
                                let mut iter = data[..2].iter();
                                dlength = *iter.next().unwrap_or(&0) as usize;
                                let typ = *iter.next().unwrap_or(&0);
                                let mut iter_desc = data[..dlength].iter();
                                match EndpointDescriptor::new(&mut iter_desc) {
                                    Some(endpoint) => {
                                        iface.endpoints.push(endpoint);
                                        epoints-=1;
                                    },
                                    None => {
                                        eprintln!("{}:{} Could not parser EndpointDescriptor for {:02X?}", bus, address, &data[..dlength]);
                                    }
                                }
                            }
                            conf.interfaces.push(iface);
                        },
                        None => {
                            eprintln!("{}:{} Could not parser InterfaceDescriptor for {:?}", bus, address, &data[..dlength]);
                        }
                    },
                    typ => eprintln!("Uknown descriptor {:02X}", typ),
                }
                data = &data[dlength..];
            }
            desc.configurations.push(conf);
        }
        println!("{}:{}\n{}", bus, address, desc);

        if data.len() > 0 {
            println!("rest: {:02X?}", data);
        }
    }

}

fn main() {
    let mut usb = LinuxUsbDevices::new();
    usb.enumerate(Path::new("/dev/bus/usb/"));
}
