use super::constants::*;
use crate::UsbDevice;
use nix::*;
use std::collections::HashMap;
use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::ptr;
#[macro_export]
macro_rules! ioctl_read_ptr {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        pub unsafe fn $name(fd: nix::libc::c_int,
                            data: *const $ty)
                            -> nix::Result<nix::libc::c_int> {
            convert_ioctl_res!(nix::libc::ioctl(fd, request_code_read!($ioty, $nr, ::std::mem::size_of::<$ty>()) as nix::sys::ioctl::ioctl_num_type, data))
        }
    )
}

#[macro_export]
macro_rules! ioctl_readwrite_ptr {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
            pub unsafe fn $name(fd: nix::libc::c_int,
                                data: *mut $ty)
                                -> nix::Result<nix::libc::c_int> {
                                    convert_ioctl_res!(nix::libc::ioctl(fd, request_code_readwrite!($ioty, $nr, ::std::mem::size_of::<$ty>()) as nix::sys::ioctl::ioctl_num_type, data))
            }
    )
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
pub struct UsbFsSetInterface {
    interface: u32,
    alt_setting: u32,
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

#[derive(Clone, Debug)]
pub struct ControlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    timeout: u32,
    data: Vec<u8>,
}

impl ControlTransfer {
    pub fn new(
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: Option<Vec<u8>>,
        timeout: u32,
    ) -> Self {
        // ugly but keep back compability for now
        let data = data.unwrap_or_else(|| Vec::new());
        let length = data.len() as u16;
        ControlTransfer {
            request_type,
            request,
            value,
            index,
            length,
            timeout,
            data,
        }
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
    pub(crate) handle: std::fs::File,
    claims: Vec<u32>,
    capabilities: u32,
    urbs: HashMap<u8, UsbFsUrb>,
    pub(crate) bus_dev: (u8, u8),
    descriptors: Option<UsbDevice>,
    read_only: bool,
}

ioctl_readwrite_ptr!(usb_control_transfer, b'U', 0, ControlTransfer);
ioctl_readwrite_ptr!(usb_bulk_transfer, b'U', 2, BulkTransfer);
ioctl_read_ptr!(usb_set_interface, b'U', 4, UsbFsSetInterface);
ioctl_write_ptr!(usb_get_driver, b'U', 8, UsbFsGetDriver);
ioctl_read_ptr!(usb_submit_urb, b'U', 10, UsbFsUrb);
ioctl_write_ptr!(usb_reapurbndelay, b'U', 13, *mut UsbFsUrb);
ioctl_read_ptr!(usb_claim_interface, b'U', 15, u32);
ioctl_read_ptr!(usb_release_interface, b'U', 16, u32);
ioctl_readwrite_ptr!(usb_ioctl, b'U', 18, UsbFsIoctl);
ioctl_read!(usb_get_capabilities, b'U', 26, u32);
ioctl_none!(usb_reset, b'U', 20);

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
            usercontext: ptr::null_mut(),
        }
    }

    pub fn control_data_as_ref<'a>(&self) -> &'a [u8] {
        let b = self.buffer_from_raw();
        if b.len() <= 8 {
            return &[];
        }
        &b[8..8 + self.actual_length as usize]
    }
}

impl Drop for UsbFsUrb {
    fn drop(&mut self) {
        if !self.buffer.is_null() {
            unsafe {
                libc::munmap(
                    self.buffer as *mut libc::c_void,
                    self.buffer_length as usize,
                );
            };
        }
    }
}

impl fmt::Display for UsbFsUrb {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "type: 0x{:02X}", self.typ)?;
        writeln!(f, "endpoint: 0x{:02X}", self.endpoint)?;
        writeln!(f, "status: 0x{0:08X} == {0}", self.status)?;
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

// should be put in another file outside os?...
pub trait UsbTransfer<T> {
    fn buffer_from_raw_mut<'a>(&self) -> &'a mut [u8];
    fn buffer_from_raw<'a>(&self) -> &'a [u8];
}

pub trait UsbCoreTransfer<T> {
    fn new_bulk(&mut self, ep: u8, size: usize) -> io::Result<T>;
    fn new_interrupt(&mut self, ep: u8, size: usize) -> io::Result<T>;
    fn new_isochronous(&mut self, ep: u8, size: usize) -> io::Result<T>;
    fn new_control(&mut self, ep: u8, ctl: ControlTransfer) -> io::Result<T>;
}

impl UsbTransfer<UsbFsUrb> for UsbFsUrb {
    fn buffer_from_raw_mut<'a>(&self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.buffer, self.buffer_length as usize) }
    }

    fn buffer_from_raw<'a>(&self) -> &'a [u8] {
        unsafe {
            if (self.endpoint & ENDPOINT_IN) == ENDPOINT_IN {
                std::slice::from_raw_parts(self.buffer, self.actual_length as usize)
            } else {
                std::slice::from_raw_parts(self.buffer, self.buffer_length as usize)
            }
        }
    }
}

impl UsbCoreTransfer<UsbFsUrb> for UsbFs {
    fn new_bulk(&mut self, ep: u8, size: usize) -> io::Result<UsbFsUrb> {
        let ptr = self.mmap(size)?;
        Ok(UsbFsUrb::new(USBFS_URB_TYPE_BULK, ep, ptr, size))
    }

    fn new_interrupt(&mut self, ep: u8, size: usize) -> io::Result<UsbFsUrb> {
        let ptr = self.mmap(size)?;
        Ok(UsbFsUrb::new(USBFS_URB_TYPE_INTERRUPT, ep, ptr, size))
    }

    fn new_isochronous(&mut self, ep: u8, size: usize) -> io::Result<UsbFsUrb> {
        let ptr = self.mmap(size)?;
        Ok(UsbFsUrb::new(USBFS_URB_TYPE_ISO, ep, ptr, size))
    }

    fn new_control(&mut self, ep: u8, ctl: ControlTransfer) -> io::Result<UsbFsUrb> {
        let length = 8 + ctl.length;
        let p = self.mmap(length as usize)?;
        //let p = libc::malloc(length as usize) as *mut u8;
        //if p.is_null() {
        //   return Err(std::io::Error::last_os_error());
        //}
        let p = unsafe {
            *p.add(0) = ctl.request_type;
            *p.add(1) = ctl.request;
            *p.add(2) = (ctl.value & 0x00FF) as u8;
            *p.add(3) = (ctl.value >> 8) as u8;
            *p.add(4) = (ctl.index & 0x00FF) as u8;
            *p.add(5) = (ctl.index >> 8) as u8;
            *p.add(6) = (ctl.length & 0x00FF) as u8;
            *p.add(7) = (ctl.length >> 8) as u8;
            p
        };
        for (i, byte) in ctl.data.iter().enumerate() {
            unsafe {
                *p.add(i + 8) = *byte;
            }
        }
        Ok(UsbFsUrb::new(
            USBFS_URB_TYPE_CONTROL,
            ep,
            p,
            length as usize,
        ))
    }
}

impl UsbFs {
    pub fn from_device(device: &UsbDevice) -> io::Result<UsbFs> {
        UsbFs::from_bus_device(device.bus_num, device.dev_num)
    }

    /// This is used when read file descriptor strings.
    pub fn from_bus_device_read_only(bus: u8, dev: u8) -> io::Result<UsbFs> {
        let mut res = UsbFs {
            handle: OpenOptions::new()
                .read(true)
                .write(false)
                .open(format!("/dev/bus/usb/{:03}/{:03}", bus, dev))?,
            claims: vec![],
            capabilities: 0,
            urbs: HashMap::new(),
            descriptors: None,
            bus_dev: (bus, dev),
            read_only: true,
        };

        res.descriptors();

        Ok(res)
    }

    pub fn from_bus_device(bus: u8, dev: u8) -> io::Result<UsbFs> {
        let mut res = UsbFs {
            handle: OpenOptions::new()
                .read(true)
                .write(true)
                .open(format!("/dev/bus/usb/{:03}/{:03}", bus, dev))?,
            claims: vec![],
            capabilities: 0,
            urbs: HashMap::new(),
            descriptors: None,
            bus_dev: (bus, dev),
            read_only: false,
        };

        res.descriptors();

        Ok(res)
    }

    pub(crate) fn handle(&self) -> &std::fs::File {
        &self.handle
    }

    pub fn reset(&mut self) -> io::Result<()> {
        let res = unsafe { usb_reset(self.handle.as_raw_fd()) };
        match res {
            Err(_) => Err(io::Error::last_os_error()),
            Ok(_) => Ok(()),
        }
    }

    pub fn descriptors(&mut self) -> &Option<UsbDevice> {
        if self.descriptors.is_none() {
            self.descriptors = UsbDevice::from_usbcore(self).ok();
        }

        &self.descriptors
    }

    /// This avoid copy used by enumerator
    #[allow(dead_code)]
    pub(crate) fn take_descriptors(&mut self) -> Option<UsbDevice> {
        self.descriptors.take()
    }

    pub fn capabilities(&mut self) -> io::Result<u32> {
        if self.capabilities != 0 {
            return Ok(self.capabilities);
        }

        let res = unsafe { usb_get_capabilities(self.handle.as_raw_fd(), &mut self.capabilities) };
        if res != Ok(0) {
            return Err(io::Error::last_os_error());
        }

        Ok(self.capabilities)
    }

    /// Returns latest transmitted async result or an error.
    /// Example:
    /// ```
    /// let mut urb = usb.new_bulk(1, 64);
    /// let urb = usb.async_response()
    /// ```
    /// The returned urb can be reused
    pub fn async_response(&mut self) -> io::Result<UsbFsUrb> {
        let urb: *mut UsbFsUrb = ptr::null_mut();
        let urb = unsafe {
            let _ = usb_reapurbndelay(self.handle.as_raw_fd(), &urb)
                .map_err(|_| io::Error::last_os_error())?;
            &*urb
        };
        let surb = match self.urbs.remove(&urb.endpoint) {
            Some(mut u) => {
                u.status = urb.status;
                u.actual_length = urb.actual_length;
                u
            }
            None => {
                log::error!("EP: {} not exists in hashmap?", urb.endpoint);
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
    pub fn claim_interface(&mut self, interface: u32) -> io::Result<()> {
        let driver: UsbFsGetDriver = unsafe { mem::zeroed() };
        let res = unsafe { usb_get_driver(self.handle.as_raw_fd(), &driver) };
        if res.is_ok() {
            panic!("FIXME the unload driver API is broken and need to be fixed");
        }
        mem::drop(driver);
        unsafe { usb_claim_interface(self.handle.as_raw_fd(), &interface) }
            .map_err(|_| io::Error::last_os_error())?;
        self.claims.push(interface);
        Ok(())
    }

    pub fn set_interface(&mut self, interface: u32, alt_setting: u32) -> io::Result<()> {
        let setter = UsbFsSetInterface {
            interface,
            alt_setting,
        };
        unsafe { usb_set_interface(self.handle.as_raw_fd(), &setter) }
            .map_err(|_| io::Error::last_os_error())?;
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
    pub fn release_interface(&self, interface: u32) -> io::Result<()> {
        unsafe { usb_release_interface(self.handle.as_raw_fd(), &interface) }
            .map_err(|_| io::Error::last_os_error())?;
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
    /// usb.control(ControlTransfer::new(0x21, 0x20, 0, 0, None, 1000);
    /// ```
    ///
    pub fn control(&mut self, ctrl: ControlTransfer) -> io::Result<Vec<u8>> {
        self.control_async_wait(ctrl)
    }

    ///
    /// Blocked bulk read
    /// Consider use @async_transfer() instead.
    pub fn bulk_read(&self, ep: u8, mem: &mut [u8]) -> io::Result<u32> {
        self.bulk(
            0x80 | ep,
            mem.as_mut_ptr() as *mut libc::c_void,
            mem.len() as u32,
        )
    }

    /// Blocked bulk write
    /// consider use @async_transfer() instead
    pub fn bulk_write(&self, ep: u8, mem: &[u8]) -> io::Result<u32> {
        self.bulk(
            ep & 0x7F,
            mem.as_ptr() as *mut libc::c_void,
            mem.len() as u32,
        )
    }

    fn bulk(&self, ep: u8, mem: *mut libc::c_void, length: u32) -> io::Result<u32> {
        let mut bulk = BulkTransfer {
            ep: ep as u32,
            length,
            timeout: 1,
            data: mem,
        };

        // wait what??
        let res = unsafe { usb_bulk_transfer(self.handle.as_raw_fd(), &mut bulk) }
            .map_err(|_| io::Error::last_os_error())?;
        // Note! ioctl return -1 on IO error...
        if res < 0 {
            return Err(io::Error::from_raw_os_error(
                nix::Error::last().as_errno().unwrap() as i32,
            ));
        }
        Ok(res as u32)
    }

    // FIXME error handling should be an result
    pub fn get_descriptor_string(&mut self, id: u8) -> String {
        self.get_descriptor_string_iface(0, id)
    }

    // FIXME error handling should be an result or an option
    pub fn get_descriptor_string_iface(&mut self, iface: u16, id: u8) -> String {
        if self.read_only {
            return "".into();
        }
        let vec = vec![0; 256];
        match self.control(ControlTransfer::new(
            0x80,
            0x06,
            0x0300 | id as u16,
            iface,
            Some(vec),
            100,
        )) {
            Ok(data) => {
                let mut length = data.len();
                if length % 2 != 0 || length == 0 {
                    log::error!(
                        "Alignment {} error invalid UTF-16 ignored string id {}",
                        length,
                        id
                    );
                    return "".into();
                }
                length /= 2;
                let utf =
                    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u16, length) };
                String::from_utf16_lossy(utf)
            }
            Err(e) => {
                log::error!(
                    "get_descriptor_string on {}-{} failed with {} on wIndex {}",
                    self.bus_dev.0,
                    self.bus_dev.1,
                    e,
                    id
                );
                "".to_string()
            }
        }
    }

    fn mmap(&mut self, length: usize) -> io::Result<*mut u8> {
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                length as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.handle.as_raw_fd(),
                0,
            )
        } as *mut u8;

        if ptr.is_null() {
            return Err(io::Error::from_raw_os_error(
                nix::errno::Errno::ENOMEM as i32,
            ));
        }
        Ok(ptr)
    }

    /// Send a async transfer
    /// It is up to the enduser to poll the file descriptor for a result.
    pub fn async_transfer(&mut self, urb: UsbFsUrb) -> io::Result<i32> {
        let res = unsafe { usb_submit_urb(self.handle.as_raw_fd(), &urb) }
            .map_err(|_| io::Error::last_os_error())?;
        self.urbs.insert(urb.endpoint, urb);
        Ok(res)
    }

    // FIXME use mio if feature  is enabled
    pub fn control_async_wait(&mut self, ctrl: ControlTransfer) -> io::Result<Vec<u8>> {
        let mut timeout = ctrl.timeout + 1;
        let asc = self.new_control(0, ctrl)?;
        self.async_transfer(asc)?;
        let urb: UsbFsUrb;
        loop {
            match self.async_response() {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if timeout == 0 {
                        return Err(e);
                    }
                    timeout -= 1;
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
                Ok(d) => {
                    urb = d;
                    break;
                }
            }
        }
        let data = Vec::from(urb.control_data_as_ref());
        Ok(data)
    }
}

impl Drop for UsbFs {
    fn drop(&mut self) {
        for claim in &self.claims {
            if self.release_interface(*claim).is_ok() {};
        }
    }
}
