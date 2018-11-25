use nix::*;
use os::linux::enumerate::UsbDevice;
use std::ffi::CString;
use std::fs::OpenOptions;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::ptr;
use mio::{Event, Ready, Poll, PollOpt, Token};
use mio::event::Evented;
use mio::unix::EventedFd;
use std::io;
use std::slice;
use std::result::Result;
use std::collections::HashMap;
use std::fmt;
impl Evented for UsbFs {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.handle.as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.handle.as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        EventedFd(&self.handle.as_raw_fd()).deregister(poll)
    }
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

const USBFS_CAP_ZERO_PACKET: u8 = 0x01;
const USBFS_CAP_BULK_CONTINUATION: u8 = 0x02;
const USBFS_CAP_NO_PACKET_SIZE_LIM: u8 = 0x04;
const USBFS_CAP_BULK_SCATTER_GATHER: u8 = 0x08;
const USBFS_CAP_REAP_AFTER_DISCONNECT: u8 = 0x10;
const USBFS_CAP_MMAP: u8 = 0x20;
const USBFS_CAP_DROP_PRIVILEGES: u8 = 0x40;

const USBFS_URB_TYPE_ISO: u8 = 0;
const USBFS_URB_TYPE_INTERRUPT: u8 = 1;
const USBFS_URB_TYPE_CONTROL: u8 = 2;
const USBFS_URB_TYPE_BULK: u8 = 3;

const USBFS_URB_FLAGS_SHORT_NOT_OK: u32 = 0x01;
const USBFS_URB_FLAGS_ISO_ASAP: u32 = 0x02;
const USBFS_URB_FLAGS_BULK_CONTINUATION: u32 = 0x04;
const USBFS_URB_FLAGS_ZERO_PACKET: u32 = 0x40;
const USBFS_URB_FLAGS_NO_INTERRUPT: u32 = 0x80;

#[repr(C)]
pub struct UsbFsIsoPacketSize {
    length: u32,
    actual_length: u32,
    status: u32,
}

#[repr(C)]
pub struct UsbFsGetDriver {
    interface: i32,
    driver: [libc::c_char; 256],
}

#[repr(C)]
pub struct UsbFsIoctl {
    interface: i32,
    code: i32,
    data: *mut libc::c_void,
}

#[repr(C)]
union UrbUnion {
    number_of_packets: i32,
    stream_id: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct UsbFsUrb {
    typ: u8,
    endpoint: u8,
    status: i32,
    flags: u32,
    pub buffer: *mut u8,
    pub buffer_length: i32,
    pub actual_length: i32,
    start_frame: i32,
    // FIXMEUNION
    stream_id: i32,
    // UNION end...
    error_count: i32,
    signr: u32,
    usercontext: *mut libc::c_void
}

#[repr(C)]
pub struct ControlTransfer {
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
pub struct BulkTransfer {
    ep: u32,
    length: u32,
    timeout: u32,
    data: *mut libc::c_void
}

pub struct UsbFs {
    handle: std::fs::File,
    claims: Vec<u32>,
    capabilities: u32,
    urbs: HashMap<u8, UsbFsUrb>
    //urbs: Vec<UsbFsUrb>
}

pub struct UrbPtr {
    ptr: *mut UsbFsUrb
}

ioctl_readwrite_ptr!(usb_control_transfer, b'U', 0, ControlTransfer);
ioctl_readwrite_ptr!(usb_bulk_transfer, b'U', 2, BulkTransfer);
ioctl_write_ptr!(usb_get_driver, b'U', 8, UsbFsGetDriver);
ioctl_read_ptr!(usb_submit_urb, b'U', 10, UsbFsUrb);
ioctl_write_ptr!(usb_reapurbndelay, b'U', 13, *mut UsbFsUrb);
ioctl_read_ptr!(usb_claim_interface, b'U', 15, u32);
ioctl_read_ptr!(usb_release_interface, b'U', 16, u32);
ioctl_readwrite_ptr!(usb_ioctl, b'U', 18, UsbFsIoctl);
ioctl_read!(usb_get_capabilities, b'U', 26, u32);

impl UsbFsUrb {
    pub fn new(ep: u8, ptr: *mut u8, length: usize) -> Self {
        UsbFsUrb {
            typ: USBFS_URB_TYPE_BULK,
            endpoint: ep,
            status: 0,
            flags: 0,
            buffer: ptr,// as *mut libc::c_void,
            buffer_length: length as i32,
            actual_length: 0 as i32,
            start_frame: 0,
            stream_id: 0,
            error_count: 0,
            signr: 0,
            usercontext: ptr as *mut libc::c_void
        }
    }
 
    pub fn get_slice<'a>(&self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.buffer, self.buffer_length as usize) }
    }
}

impl fmt::Display for UsbFsUrb {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "type: 0x{:02X}", self.typ);
        writeln!(f, "endpoint: 0x{:02X}", self.endpoint);
        writeln!(f, "status: 0x{:08X}", self.status);
        writeln!(f, "flags: 0x{:08X}", self.flags);
        writeln!(f, "buffer: {:X?}", self.buffer);
        writeln!(f, "buffer_length: {}", self.buffer_length);
        writeln!(f, "actual_length: {}", self.actual_length);
        writeln!(f, "start_frame: {}", self.start_frame);
        writeln!(f, "stream_id: {}", self.stream_id);
        writeln!(f, "signr: {}", self.signr);
        writeln!(f, "usercontext: {:X?}", self.usercontext)
    }
}


impl UsbFs {
   pub fn from_device(device: &UsbDevice) -> Result<UsbFs, io::Error> {
        let mut res = UsbFs {
            handle: OpenOptions::new()
                .read(true)
                .write(true)
                .open(format!(
                    "/dev/bus/usb/{:03}/{:03}",
                    device.bus, device.address
                ))?,
            claims: vec![],
            capabilities: 0,
            urbs: HashMap::new(),
        };

        res.capabilities();

        Ok(res)
    }

    pub fn capabilities(&mut self) -> Result<u32, nix::Error> {
        if self.capabilities != 0 {
            return Ok(self.capabilities);
        }
        let res = unsafe { usb_get_capabilities(self.handle.as_raw_fd(), &mut self.capabilities) };
        if res != Ok(0) {
            return Err(nix::Error::Sys(nix::errno::Errno::last()));
        }

        Ok(self.capabilities)
    }

    pub fn async_response(&mut self, e: Event) -> Result<(), nix::Error> {
        let urb: UsbFsUrb = unsafe { mem::zeroed() };
        let urb = Box::into_raw(Box::new(urb));
        println!("Pb {:?}", urb);
        let res = unsafe { usb_reapurbndelay(self.handle.as_raw_fd(), &urb) }.unwrap();
        println!("Pa {:?}", urb);
        let urb = unsafe { Box::from_raw(urb) };
        println!("p {:?}", urb);

        std::mem::forget(urb);
         println!("event {:?}", e);
        for (ep, urb) in &self.urbs {
            let ptr = Box::into_raw(Box::new(urb));
            println!("Pb {:?}", ptr);
             println!("{} {}", ep, urb);
            let mem = urb.get_slice();
            println!(
                "As string: {}",
                String::from_utf8_lossy(&mem));
        }

        // We will leak here :/
 //      println!("GOT\n {:?}", urb);
        Ok(())
    }

    pub fn claim_interface(&mut self, interface: u32) -> Result<(), nix::Error> {
        let driver: UsbFsGetDriver = unsafe { mem::zeroed() };
        let res = unsafe { usb_get_driver(self.handle.as_raw_fd(), &driver) };
        let driver_name = unsafe { CString::from_raw(driver.driver.to_vec().as_mut_ptr()) };
        let driver_name = driver_name.to_str().unwrap_or("");
        println!("get_driver {:?} get_driver: {:?}", res, driver_name);
        if driver_name != "usbfs" {
            let mut disconnect: UsbFsIoctl = unsafe { mem::zeroed() };
            disconnect.interface = interface as i32;
            // Disconnect driver
            disconnect.code = request_code_none!(b'U', 22) as i32;
            let res = unsafe { usb_ioctl(self.handle.as_raw_fd(), &mut disconnect) }?;
            println!("disconnect {:?}", res);
        }

        let res = unsafe { usb_claim_interface(self.handle.as_raw_fd(), &interface) }?;
        self.claims.push(interface);
        println!("claim {:?}", res);
        Ok(())
    }

    pub fn release_interface(&self, interface: u32) -> Result<(), nix::Error> {
        let res = unsafe { usb_release_interface(self.handle.as_raw_fd(), &interface) }?;
        println!("release {:?}", res);
        Ok(())
    }

    pub fn control(&self) -> Result<(), nix::Error> {
        let control = ControlTransfer {
            request_type: 0x21,
            request: 0x22,
            value: 0x3,
            index: 0,
            length: 0,
            timeout: 100,
            data: Vec::new().as_mut_ptr(),
        };

        let res = unsafe { usb_control_transfer(self.handle.as_raw_fd(), &control) }?;
        println!("control {:?}", res);

        Ok(())
    }

    pub fn bulk_read(&self, ep: u8, mem: &mut [u8]) -> Result<u32, nix::Error> {
        self.bulk(
            0x80 | ep,
            mem.as_mut_ptr() as *mut libc::c_void,
            mem.len() as u32,
        )
    }

    pub fn bulk_write(&self, ep: u8, mem: &[u8]) -> Result<u32, nix::Error> {
        // TODO error if ep highest is set eg BULK_READ?
        self.bulk(
            ep & 0x7F,
            mem.as_ptr() as *mut libc::c_void,
            mem.len() as u32,
        )
    }

    fn bulk(&self, ep: u8, mem: *mut libc::c_void, length: u32) -> Result<u32, nix::Error> {
        let bulk = BulkTransfer {
            ep: ep as u32,
            length: length,
            timeout: 10,
            data: mem,
        };

        let res = unsafe { usb_bulk_transfer(self.handle.as_raw_fd(), &bulk) };
        match res {
            Ok(len) => {
                if len >= 0 {
                    return Ok(len as u32);
                } else {
                    println!(
                        "Bulk endpoint: {:02X}, error cause {:?} FIXME return Err",
                        ep, res
                    );
                    return Ok(0);
                }
            }
            Err(res) => {
                println!("Bulk endpoint: {:02X} error cause {:?}", ep, res);
            }
        }

        Ok(0)
    }

    fn mmap(&mut self, length: usize) -> Result<*mut u8, Error> {
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                length as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.handle.as_raw_fd(),
                0 as libc::off_t)

        } as *mut u8;

        if ptr == ptr::null_mut() {
            return Err(nix::Error::Sys(nix::errno::Errno::last()));
        }
        println!("{:X?}", ptr);
        Ok(ptr)
    }

    pub fn new_bulk(&mut self, ep: u8, length: usize) -> Result<UsbFsUrb, nix::Error> {
        let ptr = self.mmap(length)?;
        Ok(UsbFsUrb::new(ep, ptr, length))
    }

    pub fn async_transfer(&mut self, urb: UsbFsUrb) -> Result<i32, nix::Error> {
        println!("len {} {:02X?}", urb.buffer_length, urb.buffer);
        let res = unsafe { usb_submit_urb(self.handle.as_raw_fd(), &urb) }?;
        println!("res is: {}", res);
        self.urbs.insert(urb.endpoint, urb);
        Ok(res)
    }
}

impl Drop for UsbFs {
    fn drop(&mut self) {
        for claim in &self.claims {
            if self.release_interface(*claim).is_ok() {};
        }
    }
}
