extern crate nix;
extern crate signal_hook;
mod descriptors;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::slice;
use nix::*;
use nix::sys::ioctl;
use std::ffi::CString;
use std::io;
use std::fs::{self,DirEntry, File};
use std::path::Path as Path;
use std::io::prelude::*;
use std::os::unix::io::AsRawFd;
use std::fs::OpenOptions;
use std::slice::Iter;
use std::fmt;
use std::thread;
use std::mem;
use std::fmt::Debug;
use std::time::Duration;

use descriptors::device::Device;
use descriptors::configuration::Configuration;
use descriptors::interface::Interface;
use descriptors::endpoint::Endpoint;
use descriptors::descriptor::{Descriptor, DescriptorType};
struct LinuxUsbDevice {
    bus: u8,
    address: u8,
    device: Device
}

struct LinuxUsbEnumerate {
    usb_devices: Vec<LinuxUsbDevice>,
}

impl fmt::Display for LinuxUsbDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}\n{}", self.bus, self.address, self.device)
    }
}

impl LinuxUsbDevice {
    fn new(bus: u8, address: u8, iter: &mut Iter<u8>) -> Option<Self> {
        Some(LinuxUsbDevice {
            bus: bus,
            address: address,
            device: Device::new(iter)?
        })
    }
}

impl LinuxUsbEnumerate {
    fn new() -> Self {
        LinuxUsbEnumerate { usb_devices: vec![]}
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
        match Configuration::new(iter_desc) {
            Some(conf) => usb.device.configurations.push(conf),
            None => eprintln!("Could not parse Configuration descriptor {:02X?} for {}:{}", iter_desc, usb.bus, usb.address)
        };
    }

    fn add_interface(&self, usb: &mut LinuxUsbDevice, iter_desc: &mut Iter<u8>) {
        let mut configuration = usb.device.configurations.last_mut().unwrap();
        match Interface::new(iter_desc) {
            Some(iface) => configuration.interfaces.push(iface),
            None => eprintln!("Could not parse Interface descriptor {:02X?} for {}:{}", iter_desc, usb.bus, usb.address)
        };
    }

    fn add_endpoint(&self, usb: &mut LinuxUsbDevice, iter_desc: &mut Iter<u8>) {
        let configuration = usb.device.configurations.last_mut().unwrap();
        let endpoints = &mut configuration.interfaces.last_mut().unwrap().endpoints;
        match Endpoint::new(iter_desc) {
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

#[repr(C)]
struct ControlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    timeout: u32,
    data: *mut libc::c_void
}

// Sync bulk transfer
#[derive(Debug)]
#[repr(C)]
struct BulkTransfer {
    ep: u32,
    length: u32,
    timeout: u32,
    data: *mut libc::c_void
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
        let driver_name = driver_name.to_str().unwrap_or("");
        println!("get_driver {:?} get_driver: {:?}", res, driver_name);
        if driver_name != "usbfs" {
            let mut disconnect: UsbFsIoctl = unsafe { mem::zeroed() };
            disconnect.interface = interface as i32;
            // Disconnect driver
            disconnect.code = request_code_none!(b'U', 22) as i32;
            let res = unsafe { usb_ioctl(self.handle.as_raw_fd(), &mut disconnect) };
            println!("disconnect {:?}", res);
        }

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

    fn control(&self) -> Result<()> {
        ioctl_readwrite_ptr!(usb_control_transfer, b'U', 0, ControlTransfer);
        let control = ControlTransfer {
            request_type: 0x21,
            request: 0x22,
            value: 0x3,
            index: 0,
            length: 0,
            timeout: 100,
            data: Vec::new().as_mut_ptr()
        };

        let res = unsafe { usb_control_transfer(self.handle.as_raw_fd(), &control) };
        println!("control {:?}", res);
 
        Ok(())
    }

    fn bulk_read(&self, ep: u8, mem: &mut [u8]) -> Result<u32> {
        self.bulk(0x80 | ep, mem.as_mut_ptr() as *mut libc::c_void, mem.len() as u32)
    }

    fn bulk_write(&self, ep: u8, mem: &[u8]) -> Result<u32> {
        // TODO error if ep highest is set eg BULK_READ?
        self.bulk(ep & 0x7F, mem.as_ptr() as *mut libc::c_void, mem.len() as u32)
    }

    fn bulk(&self, ep: u8, mem: *mut libc::c_void, length: u32) -> Result<u32> {
        ioctl_readwrite_ptr!(usb_bulk_transfer, b'U', 2, BulkTransfer);
        let bulk = BulkTransfer {
            ep: ep as u32,
            length: length,
            timeout: 10,
            data: mem
        };

        let res = unsafe { usb_bulk_transfer(self.handle.as_raw_fd(), &bulk) };
        match res {
            Ok(len) => {
                if len >= 0 {
                    return Ok(len as u32);
                } else {
                    println!("Bulk endpoint: {:02X}, error cause {:?} FIXME return Err", ep, res);
                    return Ok(0);
                }
            },
            Err(res) => {
                println!("Bulk endpoint: {:02X} error cause {:?}", ep, res);
            }
        }

        Ok(0)
    }

    fn bulk_read_async(&self, ep: u32, length: u32) -> Result<()> {
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

    let mut usb = LinuxUsbEnumerate::new();
    usb.enumerate(Path::new("/dev/bus/usb/"));

    let device = usb.get_device_from_bus(3, 5).expect("Could not get device");
    println!("{}", device);

    let mut usb = UsbFsDriver::from_device(&device).expect("FIXME actually cant fail");
    println!("Capabilities: 0x{:02X?}", usb.capabilities().unwrap());
    usb.claim_interface(0);
    usb.control();
    let mut mem: [u8; 64] = [0; 64];
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    println!("{} data {:?}", len, &mem[0..len as usize]);
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    println!("{} data: {:?}", len, &mem[0..len as usize]);
    let len = usb.bulk_write(1, "$".to_string().as_bytes()).unwrap_or(0);
    println!("{} sent data", len);
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    println!("{} data: {:?}", len, &mem[0..len as usize]);
    loop {
        if term.load(Ordering::Relaxed) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    println!("Drop dead");
}
