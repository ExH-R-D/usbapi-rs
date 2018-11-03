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

struct Descriptor {
    descriptor: Vec<u8>
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
    bus: u8,
    address: u8,
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
        let mut d = format!("{}:{}\n", self.bus, self.address);
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

impl Descriptor {
    fn new(data: Vec<u8>) -> Self {
        Descriptor { descriptor: data }
    }
}

impl Iterator for Descriptor {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Vec<u8>> {
        if self.descriptor.len() < 1 {
            return None
        }

        let dlength = self.descriptor[0] as usize;
        let give = self.descriptor[..dlength].to_vec();
        self.descriptor = self.descriptor[dlength..].to_vec();
        Some(give)
    }
}

impl DeviceDescriptor {
    fn new(bus: u8, address: u8, iter: &mut Iter<u8>) -> Option<Self> {
        Some(DeviceDescriptor {
            bus: bus,
            address: address,
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
        // FIXME better recurive checks. Should probabdly stop if uknown
        for entry in fs::read_dir(dir).expect("Can't acces usbpath?") {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => { eprintln!("{}", e); continue; },
            };
            let path = entry.path();
            if path.is_dir() {
                self.enumerate(&path);
            } else {
                let bus: u8 = path.parent().unwrap().file_name().unwrap().to_string_lossy().parse::<u8>().expect("Something is broken could not parse as u8");
                let address: u8 = path.file_name().unwrap().to_string_lossy().parse::<u8>().expect("Something is smoking could not parse dirname");
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
        let mut device = DeviceDescriptor::new(bus, address, &mut device.iter()).expect("Could not add DeviceDescriptor");
        for current in desc {
            // still unhappy with my implemention could probably be done better...
            let typ = current[1];
            let t = match typ {
                2 => {
                    self.add_configuration(&mut device, &mut current.iter())
                }
                3 => {
                    self.add_interface(&mut device, &mut current.iter())
                }
                 _ => {
                    println!("FIXME typ {} {:02X?}", typ, current);
                    continue;
                }
            };
            println!("{}:{} {}", bus, address, device);
        }
        /*
        let mut dlength = *iter.next().unwrap_or(&0) as usize;
        let mut iter_desc = data[..dlength].iter();
        let mut device = DeviceDescriptor::new(&mut iter_desc).expect("Could not add DeviceDescriptor");
        // Loosing pointer to original Vec here but ok since no use anymore
        let mut data = &data[dlength..];
        while data.len() > 0 {
            println!("{:2X?} dlength: {}", data, dlength);
            let mut iter = data[..2].iter();
            println!("head {:02X?}", iter);
            dlength = *iter.next().unwrap_or(&0) as usize;
            let typ = *iter.next().unwrap_or(&0);
            let mut iter_desc = data[..dlength].iter();
            match typ as u8{
                2 => self.add_configuration(&mut device, &mut iter_desc),
                typ => {
                    println!("FIXME typ {} {:02X?}", typ, iter_desc);
                    println!("FIXME typ {} {:02X?}", typ, data);
                }
            };
            data = &data[dlength..];
        }
        */

    }

    fn add_configuration(&self, device: &mut DeviceDescriptor, iter_desc: &mut Iter<u8>) {
        match ConfigurationDescriptor::new(iter_desc) {
            Some(conf) => device.configurations.push(conf),
            None => eprintln!("Could not parse Configuration descriptor {:02X?}", iter_desc)
        };
    }

    fn add_interface(&self, device: &mut DeviceDescriptor, iter_desc: &mut Iter<u8>) {
        let i = device.configurations.len() as usize;
        match InterfaceDescriptor::new(iter_desc) {
            Some(iface) => device.configurations[i - 1].interfaces.push(iface),
            None => eprintln!("Could not parse Configuration descriptor {:02X?}", iter_desc)
        };
    }

    fn add_endpoints(&self, device: &mut DeviceDescriptor, iter_desc: &mut Iter<u8>) {
        let conf_id = device.configurations.len() as usize;
        let iface_id = device.configurations[conf_id].interfaces.len() as usize;

        match EndpointDescriptor::new(iter_desc) {
            Some(endpoint) => {
                let conf = &device.configurations[conf_id];
                //conf.interfaces[iface_id].push(endpoint);
            },
            None => eprintln!("Could not parse Configuration descriptor {:02X?}", iter_desc)
        };
    }

/*
    pub fn add_interface(&self, mut configuration: ConfigurationDescriptor, mut data: Vec<u8>) {
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
    println!("{}:{}\n{}", bus, address, desc);

    if data.len() > 0 {
        println!("rest: {:02X?}", data);
    }
    */
}

fn main() {
    let mut usb = LinuxUsbDevices::new();
    usb.enumerate(Path::new("/dev/bus/usb/"));
}
