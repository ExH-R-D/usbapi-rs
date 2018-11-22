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
    buffer: *mut libc::c_void,
    buffer_length: i32,
    actual_length: i32,
    start_frame: i32,
    // FIXMEUNION
    stream_id: i32,
    // UNION end...
    error_count: i32,
    signr: u32,
    usercontext: *mut libc::c_void,
}

#[repr(C)]
pub struct ControlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    timeout: u32,
    data: *mut libc::c_void,
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
    ptr: *mut libc::c_void,
    urbs: Vec<UsbFsUrb>
}

ioctl_readwrite_ptr!(usb_control_transfer, b'U', 0, ControlTransfer);
ioctl_readwrite_ptr!(usb_bulk_transfer, b'U', 2, BulkTransfer);
ioctl_write_ptr!(usb_get_driver, b'U', 8, UsbFsGetDriver);
ioctl_read_ptr!(usb_submit_urb, b'U', 10, UsbFsUrb);
ioctl_read_ptr!(usb_claim_interface, b'U', 15, u32);
ioctl_read_ptr!(usb_release_interface, b'U', 16, u32);
ioctl_readwrite_ptr!(usb_ioctl, b'U', 18, UsbFsIoctl);
ioctl_read!(usb_get_capabilities, b'U', 26, u32);
impl UsbFs {
    pub fn from_device(device: &UsbDevice) -> Result<UsbFs> {
        let mut res = UsbFs {
            handle: OpenOptions::new()
                .read(true)
                .write(true)
                .open(format!(
                    "/dev/bus/usb/{:03}/{:03}",
                    device.bus, device.address
                ))
                .expect("FIXME should return error"),
            claims: vec![],
            capabilities: 0,
            ptr: ptr::null_mut(),
            urbs: vec!()
        };

        res.capabilities();
        res.mmap(64);

        Ok(res)
    }

    pub fn capabilities(&mut self) -> u32 {
        if self.capabilities != 0 {
            return self.capabilities;
        }
        let res = unsafe { usb_get_capabilities(self.handle.as_raw_fd(), &mut self.capabilities) };
        // FIXME return the error to upper layer error!!!
        // but got an compile error
        if res != Ok(0) {
            eprintln!("Error {:?}", res);
        }

        self.capabilities
    }

    pub fn async_response(&mut self, e: Event) {
        println!("{:?}", e);
        println!("{:?}", self.urbs);
        let ptr = self.ptr as *const u8;
        let slice = unsafe { slice::from_raw_parts(ptr, 64 as usize) };
        println!("{:?}", slice);
    }

    pub fn claim_interface(&mut self, interface: u32) -> Result<()> {
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

    pub fn release_interface(&self, interface: u32) -> Result<()> {
        let res = unsafe { usb_release_interface(self.handle.as_raw_fd(), &interface) };
        println!("release {:?}", res);
        Ok(())
    }

    pub fn control(&self) -> Result<()> {
        let control = ControlTransfer {
            request_type: 0x21,
            request: 0x22,
            value: 0x3,
            index: 0,
            length: 0,
            timeout: 100,
            data: Vec::new().as_mut_ptr(),
        };

        let res = unsafe { usb_control_transfer(self.handle.as_raw_fd(), &control) };
        println!("control {:?}", res);

        Ok(())
    }

    pub fn bulk_read(&self, ep: u8, mem: &mut [u8]) -> Result<u32> {
        self.bulk(
            0x80 | ep,
            mem.as_mut_ptr() as *mut libc::c_void,
            mem.len() as u32,
        )
    }

    pub fn bulk_write(&self, ep: u8, mem: &[u8]) -> Result<u32> {
        // TODO error if ep highest is set eg BULK_READ?
        self.bulk(
            ep & 0x7F,
            mem.as_ptr() as *mut libc::c_void,
            mem.len() as u32,
        )
    }

    fn bulk(&self, ep: u8, mem: *mut libc::c_void, length: u32) -> Result<u32> {
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

    fn mmap(&mut self, length: u16) {
        if self.ptr == ptr::null_mut() {
            let mut ptr = unsafe {
                let ptr = libc::mmap(
                    ptr::null_mut(),
                    length as libc::size_t,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    self.handle.as_raw_fd(),
                    0 as libc::off_t,
                    );

                if ptr == libc::MAP_FAILED {
                    Err(io::Error::last_os_error())
                } else {
                    let mut ptr = ptr as *mut u8;
                    Ok(ptr)
                }
            }.unwrap();

            let slice = unsafe { slice::from_raw_parts_mut(ptr, length as usize) };
            slice[0] = '$' as u8;
            self.ptr = ptr as *mut libc::c_void;
        }
        println!("{:X?}", self.ptr);
    }

    pub fn async_transfer(&mut self, ep: u8, length: u16) -> Result<u8> {
        let urb = UsbFsUrb {
            typ: USBFS_URB_TYPE_BULK,
            endpoint: ep,
            status: 0,
            flags: 0,
            buffer: self.ptr as *mut libc::c_void,
            buffer_length: length as i32,
            actual_length: 0 as i32,
            start_frame: 0,
            stream_id: 0,
            error_count: 0,
            signr: 0,
            usercontext: self.ptr as *mut libc::c_void,
        };

        println!("len {} {:02X?}", urb.buffer_length, urb.buffer);
        let res = unsafe { usb_submit_urb(self.handle.as_raw_fd(), &urb) };
        match res {
            Ok(len) => {
                if len >= 0 {
                    self.urbs.push(urb);
                    return Ok(0);
                } else {
                    println!("URB: {:02X}, error cause {:?} FIXME return Err", ep, res);
                    return Ok(0);
                }
            }
            Err(res) => {
                println!("URB {:02X} error cause {:?}", ep, res);
            }
        }

        Ok(0)
    }
}

impl Drop for UsbFs {
    fn drop(&mut self) {
        for claim in &self.claims {
            if self.release_interface(*claim).is_ok() {};
        }
    }
}
