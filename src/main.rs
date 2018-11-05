extern crate nix;
extern crate signal_hook;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use nix::*;
use nix::sys::ioctl;
use std::ffi::CString;
use std::slice::Iter;
use std::io;
use std::fs::{self,DirEntry, File};
use std::path::Path as Path;
use std::io::prelude::*;
use std::os::unix::io::AsRawFd;
use std::fs::OpenOptions;
use std::fmt;
use std::thread;
use std::mem;
use std::fmt::Debug;
use std::time::Duration;

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


struct UsbFsDriver {
    handle: std::fs::File,
    claims: Vec<u32>
}

impl UsbFsDriver {
    fn from_device(device: &LinuxUsbDevice) -> Result<UsbFsDriver> {
        Ok(UsbFsDriver {
            handle: OpenOptions::new().read(true).write(true).open(format!("/dev/bus/usb/{:03}/{:03}", device.bus, device.address)).expect("FIXME should return error"),
            claims: vec![]
        })
    }

    fn capabilities(&self) -> Result<u32> {
        ioctl_read!(usb_get_capabilities, b'U', 26, u32);
        let mut cap = 0;
        let res = unsafe { usb_get_capabilities(self.handle.as_raw_fd(), &mut cap) };
        // FIXME return the error to upper layer error!!!
        // but got an compile error
        if res != Ok(0) {
            eprintln!("Error {:?}", res);
        }

        Ok(cap)
    }

    fn claim_interface(&mut self, interface: u32) -> Result<()> {
        ioctl_write_ptr!(usb_get_driver, b'U', 8, UsbFsGetDriver);
        ioctl_readwrite_ptr!(usb_ioctl, b'U', 18, UsbFsIoctl);
        ioctl_read_ptr!(usb_claim_interface, b'U', 15, u32);
        let mut driver: UsbFsGetDriver = unsafe { mem::zeroed() };
        let res = unsafe { usb_get_driver(self.handle.as_raw_fd(), &driver) };
        let driver_name = unsafe { CString::from_raw(driver.driver.to_vec().as_mut_ptr()) };
        println!("get_driver {:?} get_driver: {:?}", res, driver_name);

        let mut disconnect: UsbFsIoctl = unsafe { mem::zeroed() };
        disconnect.interface = interface as i32;
        // Disconnect driver
        disconnect.code = request_code_none!(b'U', 22) as i32;
        let res = unsafe { usb_ioctl(self.handle.as_raw_fd(), &mut disconnect) };
        println!("disconnect {:?}", res);
        let res = unsafe { usb_claim_interface(self.handle.as_raw_fd(), &interface) };
        if res == Ok(0) {
            self.claims.push(interface);
        }
        println!("claim {:?}", res);
        Ok(())
    }

    fn release_interface(&self, interface: u32) -> Result<()> {
        ioctl_read_ptr!(usb_release_interface, b'U', 16, u32);
        let res = unsafe { usb_release_interface(self.handle.as_raw_fd(), &interface) };
        println!("release {:?}", res);
        Ok(())
    }
}

impl Drop for UsbFsDriver {
    fn drop(&mut self) {
        for claim in &self.claims {
            self.release_interface(*claim);
        }
    }
}

#[repr(C)]
struct UsbFsControlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    timeout: u32,
    data: *mut libc::c_void
}

 #[repr(C)]
struct UsbFsIsoPacketSize {
    length: u32,
    actual_length: u32,
    status: u32
}

#[repr(C)]
struct UsbFsGetDriver {
    interface: i32,
    driver: [libc::c_char; 256]
}

#[repr(C)]
struct UsbFsIoctl {
    interface: i32,
    code: i32,
    data: *mut libc::c_void
}

enum urb_union {
        number_of_packets(i32),
        stream_id(u32)
}
#[repr(C)]
struct UsbFsUrb {
    typ: u8,
    endpoint: u8,
    status: u32,
    flags: u32,
    buffer: *mut libc::c_void,
	buffer_length: i32,
	actual_length: i32,
	start_frame: i32,
    union: urb_union,
    error_count: i32,
    signr: u32,
    usercontext: *mut libc::c_void,
    iso_frame_desc: UsbFsIsoPacketSize
}


#[macro_export]
macro_rules! ioctl_read_ptr {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: $crate::libc::c_int,
                            data: *const $ty)
                            -> $crate::Result<$crate::libc::c_int> {
            convert_ioctl_res!($crate::libc::ioctl(fd, request_code_read!($ioty, $nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
        }
    )
}

#[macro_export]
macro_rules! ioctl_readwrite_ptr {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
            pub unsafe fn $name(fd: $crate::libc::c_int,
                                data: *const $ty)
                                -> $crate::Result<$crate::libc::c_int> {
                                    convert_ioctl_res!($crate::libc::ioctl(fd, request_code_readwrite!($ioty, $nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
            }
    )
}


//ioctl_none!(usb_disconnect, b'U', 22);

const USBFS_URB_TYPE_ISO: u8 = 0;
const USBFS_URB_TYPE_INTERRUPT: u8 = 1;
const USBFS_URB_TYPE_CONTROL: u8 = 2;
const USBFS_URB_TYPE_BULK: u8 = 2;

fn main() {
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&term)).unwrap();
 ioctl_write_ptr!(usb_submiturb, b'U', 0, UsbFsUrb);
ioctl_write_ptr!(usb_control_transfer, b'U', 10, UsbFsControlTransfer);

    let mut usb = LinuxUsbDevices::new();
    usb.enumerate(Path::new("/dev/bus/usb/"));

    let device = usb.get_device_from_bus(3, 7).expect("Could not get device");
    println!("{}", device);

    let mut usb = UsbFsDriver::from_device(&device).expect("FIXME actually cant fail");
    println!("Capabilities: 0x{:02X?}", usb.capabilities().unwrap());
    usb.claim_interface(0);
    usb.claim_interface(0);
/*
    let mut file = OpenOptions::new().read(true).write(true).open(format!("/dev/bus/usb/{:03}/{:03}", device.bus, device.address)).expect("failed to open usb");
    let mut cap = 0;
    let res = unsafe { usb_get_capabilities(file.as_raw_fd(), &mut cap) };
    println!("res {:?} Capabilitys: 0x{:02X}", res, cap);

    let mut driver: UsbFsGetDriver = unsafe { mem::zeroed() };
    let res = unsafe { usb_get_driver(file.as_raw_fd(), &driver) };
    let driver_name = unsafe { CString::from_raw(driver.driver.to_vec().as_mut_ptr()) };
    println!("res {:?} get_driver: {:?}", res, driver_name);

    let mut claim: UsbFsIoctl = unsafe { mem::zeroed() };
    claim.interface = 0;
    claim.code = request_code_none!(b'U', 22) as i32;
    let res = unsafe { usb_ioctl(file.as_raw_fd(), &mut claim) };
    println!("res {:?} {:08X}", res, claim.code);
    let res = request_code_readwrite!(b'U', 18, 0xFFFFFF); 
    println!("res {:08X}", res);

    let mut iface = 1;
    let res = unsafe { usb_claim_interface(file.as_raw_fd(), &mut iface) };
    println!("res {:?} {}", res, iface);
   */
    loop {
        if term.load(Ordering::Relaxed) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    println!("Drop dead");
}
