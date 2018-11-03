extern crate nix;
use nix::ioctl_readwrite_buf;
use nix::ioctl_write_buf;
use nix::*;
use nix::sys::ioctl;
use std::slice::Iter;
use std::io;
use std::fs::{self,DirEntry, File};
use std::path::Path as Path;
use std::io::prelude::*;
use std::os::unix::io::AsRawFd;
use std::fmt;
use std::mem;

struct LinuxUsbDevice {
    bus: u8,
    address: u8,
    device: DeviceDescriptor
}

struct LinuxUsbDevices {
    usb_devices: Vec<LinuxUsbDevice>,
}

#[derive(Debug)]
enum DescriptorType {
    Device,
    Configuration,
    String,
    Interface,
    Endpoint,
    ClassSpecific,
    Hub,
    SS_Endpoint_Companion,
    Unknown,
}

/*impl From<DescriptorType> for u8 {
    fn from(original: DescriptorType) -> u8 {
        match original {
            DescriptorType::Device => 1,
            DescriptorType::Configuration => 2,
            DescriptorType::String => 3,
            DescriptorType::Interface => 4,
            DescriptorType::Endpoint => 5,
            DescriptorType::ClassSpecific => 0x24,
            DescriptorType::Hub => 0x29,
            DescriptorType::SS_Endpoint_Companion => 0x30,
        }
    }
}
*/


impl From<u8> for DescriptorType {
    fn from(original: u8) -> DescriptorType {
        match original {
            1 => DescriptorType::Device,
            2 => DescriptorType::Configuration,
            3 => DescriptorType::String,
            4 => DescriptorType::Interface,
            5 => DescriptorType::Endpoint,
            0x24 => DescriptorType::ClassSpecific,
            0x29 => DescriptorType::Hub,
            0x30 => DescriptorType::SS_Endpoint_Companion,
            n => DescriptorType::Unknown
        }
    }
}
// Just a used trait when create below
// Descriptors...
struct Descriptor {
    descriptor: Vec<u8>
}

struct DeviceDescriptor {
    length: u8,
    kind: u8,
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
    kind: u8,
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
    kind: u8,
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
    kind: u8,
    endpoint_address: u8,
    bm_attributes: u8,
    max_packet_size: u16,
    interval: u8
}

impl fmt::Display for LinuxUsbDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}\n{}", self.bus, self.address, self.device)
    }
}

impl fmt::Display for DeviceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = format!("bLength: {}\n", self.length);
        d+=&format!("bDescriptorType: {:?}\n", self.kind);
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
        d+=&format!("bDescriptorType: {}\n", self.kind);
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
        d+=&format!("bDescriptorType: {}\n", self.kind);
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
        d+=&format!("bDescriptorType: {}\n", self.kind);
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

impl LinuxUsbDevice {
    fn new(bus: u8, address: u8, iter: &mut Iter<u8>) -> Option<Self> {
        Some(LinuxUsbDevice {
            bus: bus,
            address: address,
            device: DeviceDescriptor::new(iter)?
        })
    }
}

impl DeviceDescriptor {
    fn new(iter: &mut Iter<u8>) -> Option<Self> {
        Some(DeviceDescriptor {
            length: *iter.next()?,
            kind: *iter.next()?,
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
            kind: *iter.next()?,
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
            kind: *iter.next()?,
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
            kind: *iter.next()?,
            endpoint_address: *iter.next()?,
            bm_attributes: *iter.next()?,
            max_packet_size: *iter.next()? as u16 | (*iter.next()? as u16) << 8,
            interval: *iter.next()?
        })
    }
}

impl LinuxUsbDevices {
    fn new() -> Self {
        LinuxUsbDevices{ usb_devices: vec![]}
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
        let mut device = LinuxUsbDevice::new(bus, address, &mut device.iter()).expect("Could not add DeviceDescriptor");
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
        self.usb_devices.push(device);
    }

    fn add_configuration(&self, usb: &mut LinuxUsbDevice, iter_desc: &mut Iter<u8>) {
        match ConfigurationDescriptor::new(iter_desc) {
            Some(conf) => usb.device.configurations.push(conf),
            None => eprintln!("Could not parse Configuration descriptor {:02X?} for {}:{}", iter_desc, usb.bus, usb.address)
        };
    }

    fn add_interface(&self, usb: &mut LinuxUsbDevice, iter_desc: &mut Iter<u8>) {
        let mut configuration = usb.device.configurations.last_mut().unwrap();
        match InterfaceDescriptor::new(iter_desc) {
            Some(iface) => configuration.interfaces.push(iface),
            None => eprintln!("Could not parse Interface descriptor {:02X?} for {}:{}", iter_desc, usb.bus, usb.address)
        };
    }

    fn add_endpoint(&self, usb: &mut LinuxUsbDevice, iter_desc: &mut Iter<u8>) {
        let configuration = usb.device.configurations.last_mut().unwrap();
        let endpoints = &mut configuration.interfaces.last_mut().unwrap().endpoints;
        match EndpointDescriptor::new(iter_desc) {
            Some(endpoint) => {
              //  let mut endpoints = &mut interfaces.endpoints;
                endpoints.push(endpoint);
            },
            None => eprintln!("Could not parse Endpoint descriptor {:02X?} for {}:{}", iter_desc, usb.bus, usb.address)
        };
    }

    fn get_device_from_bus(&self, bus: u8, address: u8) -> Option<&LinuxUsbDevice> {
        for usb in &self.usb_devices {
            if usb.bus == bus && usb.address == address {
                return Some(&usb);
            }
        };
        None
    }
}


#[repr(C)]
struct UsbFsIsoPacketSize {
    length: usize,
    actual_length: usize,
    status: usize
}

#[repr(C)]
struct UsbFsUrb {
    typ: u8,
    endpoint: u8,
    status: isize,
    flags: isize,

    buffer: *mut libc::c_void,
	buffer_length: isize,
	actual_length: isize,
	start_frame: isize,
    stream_id: isize,
    /*
	union {
		int number_of_packets;	/* Only used for isoc urbs */
		unsigned int stream_id;	/* Only used with bulk streams */
	};
*/
    error_count: isize,
    signr: usize,
    usercontext: *mut libc::c_void,
    iso_frame_desc: UsbFsIsoPacketSize
}

const USBFS_URB_TYPE_CONTROL: u8 = 2;

fn main() {
    let mut usb = LinuxUsbDevices::new();
    usb.enumerate(Path::new("/dev/bus/usb/"));

    let device = usb.get_device_from_bus(3, 4).expect("Could not get device");
    println!("{}", device);

    let file = File::open("/dev/bus/usb/003/004").expect("failed to open usb");
    ioctl_write_buf!(usb_control, 'U', 10, UsbFsUrb);
    let mut urb: UsbFsUrb = unsafe { mem::zeroed() };
    urb.typ = USBFS_URB_TYPE_CONTROL;
    urb.endpoint = 0x01;
    let res = unsafe { usb_control(file.as_raw_fd(), &[urb]); };
    println!("res {:?}", res);
}
