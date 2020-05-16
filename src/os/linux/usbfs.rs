use crate::os::linux::enumerate::UsbDevice;
use mio::event::Evented;
use mio::unix::EventedFd;
use mio::{Poll, PollOpt, Ready, Token};
use nix::*;
use std::collections::HashMap;
use std::ffi::CString;
use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::result::Result;
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
                                data: *mut $ty)
                                -> $crate::Result<$crate::libc::c_int> {
                                    convert_ioctl_res!($crate::libc::ioctl(fd, request_code_readwrite!($ioty, $nr, ::std::mem::size_of::<$ty>()) as $crate::sys::ioctl::ioctl_num_type, data))
            }
    )
}

#[allow(dead_code)]
const USBFS_CAP_ZERO_PACKET: u8 = 0x01;
#[allow(dead_code)]
const USBFS_CAP_BULK_CONTINUATION: u8 = 0x02;
#[allow(dead_code)]
const USBFS_CAP_NO_PACKET_SIZE_LIM: u8 = 0x04;
#[allow(dead_code)]
const USBFS_CAP_BULK_SCATTER_GATHER: u8 = 0x08;
#[allow(dead_code)]
const USBFS_CAP_REAP_AFTER_DISCONNECT: u8 = 0x10;
#[allow(dead_code)]
const USBFS_CAP_MMAP: u8 = 0x20;
#[allow(dead_code)]
const USBFS_CAP_DROP_PRIVILEGES: u8 = 0x40;

const USBFS_URB_TYPE_ISO: u8 = 0;
const USBFS_URB_TYPE_INTERRUPT: u8 = 1;
#[allow(dead_code)]
const USBFS_URB_TYPE_CONTROL: u8 = 2;
const USBFS_URB_TYPE_BULK: u8 = 3;

#[allow(dead_code)]
const USBFS_URB_FLAGS_SHORT_NOT_OK: u32 = 0x01;
#[allow(dead_code)]
const USBFS_URB_FLAGS_ISO_ASAP: u32 = 0x02;
#[allow(dead_code)]
const USBFS_URB_FLAGS_BULK_CONTINUATION: u32 = 0x04;
#[allow(dead_code)]
const USBFS_URB_FLAGS_ZERO_PACKET: u32 = 0x40;
#[allow(dead_code)]
const USBFS_URB_FLAGS_NO_INTERRUPT: u32 = 0x80;

#[allow(dead_code)]
pub const RECIPIENT_DEVICE: u8 = 0x00;
#[allow(dead_code)]
pub const RECIPIENT_INTERFACE: u8 = 0x01;
#[allow(dead_code)]
pub const RECIPIENT_ENDPOINT: u8 = 0x02;
#[allow(dead_code)]
pub const RECIPIENT_OTHER: u8 = 0x03;
#[allow(dead_code)]
pub const REQUEST_TYPE_STANDARD: u8 = 0x00 << 5;
#[allow(dead_code)]
pub const REQUEST_TYPE_CLASS: u8 = 0x01 << 5;
#[allow(dead_code)]
pub const ENDPOINT_IN: u8 = 0x80;
#[allow(dead_code)]
pub const ENDPOINT_OUT: u8 = 0x00;

enum RequestType {
    EndpointIN,
    RequestTypeClass,
}

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

#[allow(dead_code)]
#[repr(C)]
union UrbUnion {
    number_of_packets: i32,
    stream_id: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ControlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    timeout: u32,
    pub data: *mut u8,
}

impl ControlTransfer {
    pub fn new(
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        mut data: Vec<u8>,
        timeout: u32,
    ) -> Self {
        let length = data.capacity() as u16;
        ControlTransfer {
            request_type,
            request,
            value,
            index,
            length,
            timeout,
            data: data.as_mut_ptr(),
        }
    }

    pub fn no_data(request_type: u8, request: u8, value: u16, index: u16, timeout: u32) -> Self {
        ControlTransfer {
            request_type,
            request,
            value,
            index,
            length: 0,
            timeout,
            data: ptr::null_mut(),
        }
    }

    pub fn get_slice<'a>(&self) -> &'a [u8] {
        if self.length == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts_mut(self.data, self.length as usize) }
    }
}

// Sync bulk transfer
#[derive(Debug)]
#[repr(C)]
pub struct BulkTransfer {
    ep: u32,
    length: u32,
    timeout: u32,
    data: *mut libc::c_void,
}

pub struct UsbFs {
    handle: std::fs::File,
    claims: Vec<u32>,
    capabilities: u32,
    urbs: HashMap<u8, UsbFsUrb>,
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

#[derive(Debug)]
#[repr(C)]
pub struct UsbFsUrb {
    typ: u8,
    endpoint: u8,
    pub status: i32,
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
    usercontext: *mut u8,
}

impl UsbFsUrb {
    pub fn new(typ: u8, ep: u8, ptr: *mut u8, length: usize) -> Self {
        UsbFsUrb {
            typ,
            endpoint: ep,
            status: 0,
            flags: 0,
            buffer: ptr,
            buffer_length: length as i32,
            actual_length: 0 as i32,
            start_frame: 0,
            stream_id: 0,
            error_count: 0,
            signr: 0,
            usercontext: ptr,
        }
    }

    pub fn get_slice<'a>(&self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.buffer, self.buffer_length as usize) }
    }
}

impl fmt::Display for UsbFsUrb {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "type: 0x{:02X}", self.typ)?;
        writeln!(f, "endpoint: 0x{:02X}", self.endpoint)?;
        writeln!(f, "status: 0x{:08X}", self.status)?;
        writeln!(f, "flags: 0x{:08X}", self.flags)?;
        writeln!(f, "buffer: {:X?}", self.buffer)?;
        writeln!(f, "buffer_length: {}", self.buffer_length)?;
        writeln!(f, "actual_length: {}", self.actual_length)?;
        writeln!(f, "start_frame: {}", self.start_frame)?;
        writeln!(f, "stream_id: {}", self.stream_id)?;
        writeln!(f, "signr: {}", self.signr)?;
        writeln!(f, "usercontext: {:X?}", self.usercontext)
    }
}

// EXPERIMENTAL FIXME error should not be nix::Error
// Also it should be put in another file...
pub trait UsbTransfer<T> {
    fn buffer_from_raw_mut<'a>(&self) -> &'a mut [u8];
    fn buffer_from_raw<'a>(&self) -> &'a [u8];
}

pub trait UsbCoreTransfer<T> {
    fn new_bulk(&mut self, ep: u8, size: usize) -> Result<T, nix::Error>;
    fn new_interrupt(&mut self, ep: u8, size: usize) -> Result<T, nix::Error>;
    fn new_isochronous(&mut self, ep: u8, size: usize) -> Result<T, nix::Error>;
    // Fells akward need to rethink
}

impl UsbTransfer<UsbFsUrb> for UsbFsUrb {
    fn buffer_from_raw_mut<'a>(&self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.buffer, self.buffer_length as usize) }
    }

    fn buffer_from_raw<'a>(&self) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts(self.buffer, self.buffer_length as usize) }
    }
}

impl UsbCoreTransfer<UsbFsUrb> for UsbFs {
    fn new_bulk(&mut self, ep: u8, size: usize) -> Result<UsbFsUrb, nix::Error> {
        let ptr = self.mmap(size)?;
        Ok(UsbFsUrb::new(USBFS_URB_TYPE_BULK, ep, ptr, size))
    }

    fn new_interrupt(&mut self, ep: u8, size: usize) -> Result<UsbFsUrb, nix::Error> {
        let ptr = self.mmap(size)?;
        Ok(UsbFsUrb::new(USBFS_URB_TYPE_INTERRUPT, ep, ptr, size))
    }

    fn new_isochronous(&mut self, ep: u8, size: usize) -> Result<UsbFsUrb, nix::Error> {
        let ptr = self.mmap(size)?;
        Ok(UsbFsUrb::new(USBFS_URB_TYPE_ISO, ep, ptr, size))
    }
}

impl UsbFs {
    pub fn from_device(device: &UsbDevice) -> Result<UsbFs, io::Error> {
        UsbFs::from_bus_address(device.bus, device.address)
    }

    pub fn from_bus_address(bus: u8, address: u8) -> Result<UsbFs, io::Error> {
        let mut res = UsbFs {
            handle: OpenOptions::new()
                .read(true)
                .write(true)
                .open(format!("/dev/bus/usb/{:03}/{:03}", bus, address))?,
            claims: vec![],
            capabilities: 0,
            urbs: HashMap::new(),
        };

        res.capabilities().unwrap();

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

    /// Returns latest transmitted async result or an error.
    /// Example:
    /// ```
    /// let mut urb = usb.new_bulk(1, 64);
    /// let urb = usb.async_response()
    /// ```
    pub fn async_response(&mut self) -> Result<UsbFsUrb, nix::Error> {
        let urb: *mut UsbFsUrb = ptr::null_mut();
        let urb = unsafe {
            usb_reapurbndelay(self.handle.as_raw_fd(), &urb)?;
            &*urb
        };
        let surb = match self.urbs.remove(&urb.endpoint) {
            Some(mut u) => {
                u.status = urb.status;
                u.actual_length = urb.actual_length;
                u
            }
            None => {
                eprintln!("EP: {} not exists in hashmap?", urb.endpoint);
                UsbFsUrb::new(0xFF, urb.endpoint, ptr::null_mut(), 0)
            }
        };

        Ok(surb)
    }

    /// Claim interface
    ///
    /// * the `interface` number to claim
    ///
    /// Examples
    ///
    /// Basic usage:
    /// ```
    /// usb.claim_interface(1)
    /// ```
    ///
    pub fn claim_interface(&mut self, interface: u32) -> Result<(), nix::Error> {
        let driver: UsbFsGetDriver = unsafe { mem::zeroed() };
        let _res = unsafe { usb_get_driver(self.handle.as_raw_fd(), &driver) };
        let driver_name = unsafe { CString::from_raw(driver.driver.to_vec().as_mut_ptr()) };
        let driver_name = driver_name.to_str().unwrap_or("");
        if driver_name != "usbfs" {
            let mut disconnect: UsbFsIoctl = unsafe { mem::zeroed() };
            disconnect.interface = interface as i32;
            // Disconnect driver
            disconnect.code = request_code_none!(b'U', 22) as i32;
            let _res = unsafe { usb_ioctl(self.handle.as_raw_fd(), &mut disconnect) }?;
        }

        let _res = unsafe { usb_claim_interface(self.handle.as_raw_fd(), &interface) }?;
        self.claims.push(interface);
        Ok(())
    }

    /// Release interface
    ///
    /// * the `interface` number to claim
    ///
    /// Examples
    ///
    /// Basic usage:
    /// ```
    /// usb.release_interface(1)
    /// ```
    ///
    pub fn release_interface(&self, interface: u32) -> Result<(), nix::Error> {
        unsafe { usb_release_interface(self.handle.as_raw_fd(), &interface) }?;
        Ok(())
    }

    /// Send control
    ///
    /// * `ControlTransfer` structure.
    ///
    /// Examples
    ///
    /// Basic usage:
    /// ```
    /// usb.control(ControlTransfer::new(0x21, 0x20, 0, 0, vec!(), 100);
    /// ```
    ///
    pub fn control(&self, mut ctrl: ControlTransfer) -> Result<Vec<u8>, nix::Error> {
        let len = unsafe { usb_control_transfer(self.handle.as_raw_fd(), &mut ctrl) }?;
        if len < 0 {
            return Err(nix::Error::last());
        }
        ctrl.length = len as u16;
        let mut res = Vec::new();
        for d in ctrl.get_slice() {
            res.push(*d);
        }
        Ok(res)
    }

    /// Blocked bulk read
    /// Consider use @async_transfer() instead.
    pub fn bulk_read(&self, ep: u8, mem: &mut [u8]) -> Result<u32, nix::Error> {
        self.bulk(
            0x80 | ep,
            mem.as_mut_ptr() as *mut libc::c_void,
            mem.len() as u32,
        )
    }

    /// Blocked bulk write
    /// consider use @async_transfer() instead
    pub fn bulk_write(&self, ep: u8, mem: &[u8]) -> Result<u32, nix::Error> {
        self.bulk(
            ep & 0x7F,
            mem.as_ptr() as *mut libc::c_void,
            mem.len() as u32,
        )
    }

    fn bulk(&self, ep: u8, mem: *mut libc::c_void, length: u32) -> Result<u32, nix::Error> {
        let mut bulk = BulkTransfer {
            ep: ep as u32,
            length,
            timeout: 10,
            data: mem,
        };

        let res = unsafe { usb_bulk_transfer(self.handle.as_raw_fd(), &mut bulk) }?;
        // Note! ioctl return -1 on IO error...
        if res < 0 {
            eprintln!("Bulk endpoint: {:02X} error cause {:?}", ep, res);
            return Err(nix::Error::last());
        }
        Ok(res as u32)
    }

    #[allow(clippy::cast_ptr_alignment)]
    pub fn get_descriptor_string(&mut self, id: u8) -> String {
        let vec = Vec::with_capacity(64);
        match self.control(ControlTransfer::new(
            0x80,
            0x06,
            0x0300 | id as u16,
            0,
            vec,
            100,
        )) {
            Ok(data) => {
                let mut length = data.len();
                if length % 2 != 0 || length == 0 {
                    eprintln!(
                        "Alignment {} error invalid UTF-16 ignored string id {}",
                        length, id
                    );
                    return "Invalid UTF-16".into();
                }
                length /= 2;
                let utf =
                    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u16, length) };
                String::from_utf16_lossy(utf)
            }
            Err(e) => {
                eprintln!("get_descriptor_string failed with {} for {}", e, id);
                "".to_string()
            }
        }
    }

    fn mmap(&mut self, length: usize) -> Result<*mut u8, Error> {
        let mut ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                length as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.handle.as_raw_fd(),
                0 as libc::off_t,
            )
        } as *mut u8;

        if ptr.is_null() {
            // if mmap fail we try malloc instead
            ptr = unsafe { libc::calloc(1, length) as *mut u8 };
            if ptr.is_null() {
                return Err(nix::Error::Sys(nix::errno::Errno::ENOMEM));
            }
        }
        Ok(ptr)
    }

    /*
       /// Setup a new bulk package for async send or recieve
       /// * `ep` Endpoint
       /// * `length` max length
       /// * Returns UsbFsUrb with malloc'ed transfer buffer.
       pub fn new_bulk(&mut self, ep: u8, length: usize) -> Result<UsbFsUrb, nix::Error> {
           let ptr = self.mmap(length)?;
           let mut urb = UsbFsUrb::new_bulk(ep, length)?;
           urb.buffer = ptr as *mut libc::c_void
    ;
           Ok(urb)
       }

       /// Untested
       pub fn new_isochronous(&mut self, ep: u8, length: usize) -> Result<UsbFsUrb, nix::Error> {
           let ptr = self.mmap(length)?;
           Ok(UsbFsUrb::new(USBFS_URB_TYPE_ISO, ep, ptr, length))
       }

       /// Untested
       pub fn new_interrupt(&mut self, ep: u8, length: usize) -> Result<UsbFsUrb, nix::Error> {
           let ptr = self.mmap(length)?;
           Ok(UsbFsUrb::new(USBFS_URB_TYPE_INTERRUPT, ep, ptr, length))
       }
       */

    /// Send a async transfer
    /// It is up to thbe enduser to poll the file descriptor for a result.
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
