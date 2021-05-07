use super::usbfsurb::*;
use crate::endpoint::Endpoint;
use crate::usb_transfer::*;
use crate::TimeoutMillis;
use crate::UsbDevice;
use nix::*;
use std::ffi::CStr;
use std::io;
use std::io::Write;
use std::io::{Error, ErrorKind};
use std::mem;
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::*;
use std::ptr;
use std::time::{Duration, Instant};

const CONTROL_MAX_PACKET_SIZE: u16 = 1024;
#[macro_export]
macro_rules! ioctl_read_ptr {
    ($(#[$attr:meta])* $name:ident, $ioty:expr, $nr:expr, $ty:ty) => (
        $(#[$attr])*
        /// # Safety
        /// ioctl call need unsafe calls to C
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
            /// # Safety
            /// ioctl call need unsafe calls to C
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
    driver: [u8; 256],
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

// Sync bulk transfer
#[derive(Debug)]
#[repr(C)]
pub struct CBulkTransfer {
    ep: u32,
    length: u32,
    timeout_ms: u32,
    data: *mut libc::c_void,
}

pub struct UsbFs {
    pub(crate) handle: std::fs::File,
    claims: Vec<u32>,
    capabilities: u32,
    transfers: Vec<TransferKind>,
    pub(crate) bus_dev: (u8, u8),
    descriptors: Option<UsbDevice>,
    read_only: bool,
}

ioctl_readwrite_ptr!(usb_control_transfer, b'U', 0, ControlTransfer);
ioctl_readwrite_ptr!(usb_bulk_transfer, b'U', 2, CBulkTransfer);
ioctl_read_ptr!(usb_set_interface, b'U', 4, UsbFsSetInterface);
ioctl_write_ptr!(usb_get_driver, b'U', 8, UsbFsGetDriver);
ioctl_read_ptr!(usb_submit_urb, b'U', 10, UsbFsUrb);
ioctl_write_ptr!(usb_reapurbndelay, b'U', 13, *mut UsbFsUrb);
ioctl_read_ptr!(usb_claim_interface, b'U', 15, u32);
ioctl_read_ptr!(usb_release_interface, b'U', 16, u32);
ioctl_readwrite_ptr!(usb_ioctl, b'U', 18, UsbFsIoctl);
ioctl_read!(usb_get_capabilities, b'U', 26, u32);
ioctl_none!(usb_reset, b'U', 20);
ioctl_read!(usb_clear_halt, b'U', 21, u32);

impl UsbCoreDriver for UsbFs {
    // Create a new BulkTransfer for reading
    // buffer_capacity tells how much we want to allocate for the read buffer
    // Example:
    // ```let transfer = usb.new_bulk_in(0x01, 64)?;
    // usb.submit_bulk(transfer);
    // ```
    fn new_bulk_in(&mut self, ep: u8, buffer_capacity: usize) -> io::Result<BulkTransfer> {
        let ptr = self.mmap(buffer_capacity)?;
        Ok(BulkTransfer::input(ep, ptr, buffer_capacity, Self::munmap))
    }

    // Create a new BulkTransfer for reading
    // buffer_capacity tells how much we want to allocate for the read buffer
    // Example:
    // ```let transfer = usb.new_bulk_in(0x01, 64)?;
    // usb.submit_bulk(transfer);
    // ```
    fn new_bulk_out(&mut self, ep: u8, buffer_capacity: usize) -> io::Result<BulkTransfer> {
        let ptr = self.mmap(buffer_capacity)?;
        Ok(BulkTransfer::output(ep, ptr, buffer_capacity, Self::munmap))
    }

    fn new_control(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        length: u16,
    ) -> io::Result<ControlTransfer> {
        if length > CONTROL_MAX_PACKET_SIZE {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Data bigger than {} is not supported on the control endpoint",
                    CONTROL_MAX_PACKET_SIZE
                ),
            ));
        }

        let capacity_length = length as usize + 8;
        let p = self.mmap(capacity_length)?;
        let mut ctrl = ControlTransfer::new(p, capacity_length as u16, 0, Self::munmap);
        ctrl.write_all(&[
            request_type,
            request,
            (value & 0x00FF) as u8,
            (value >> 8) as u8,
            (index & 0x00FF) as u8,
            (index >> 8) as u8,
            (length & 0x00FF) as u8,
            (length >> 8) as u8,
        ])?;

        Ok(ctrl)
    }
}

impl UsbFs {
    pub fn from_device(device: &UsbDevice) -> io::Result<UsbFs> {
        UsbFs::from_bus_device(device.bus_num, device.dev_num)
    }

    /// This is used when read file descriptor strings.
    pub fn from_bus_device_read_only(bus: u8, dev: u8) -> io::Result<UsbFs> {
        use nix::fcntl::OFlag;
        let path = format!("/dev/bus/usb/{:03}/{:03}", bus, dev);
        let path = std::path::Path::new(&path);
        let handle = nix::fcntl::open(
            path,
            OFlag::O_RDONLY | OFlag::O_NOCTTY | OFlag::O_NONBLOCK,
            nix::sys::stat::Mode::empty(),
        )
        .map_err(|_| io::Error::last_os_error())?;

        let mut res = UsbFs {
            handle: unsafe { std::fs::File::from_raw_fd(handle) },
            claims: vec![],
            capabilities: 0,
            transfers: Vec::new(),
            descriptors: None,
            bus_dev: (bus, dev),
            read_only: true,
        };

        res.descriptors();

        Ok(res)
    }

    pub fn from_bus_device(bus: u8, dev: u8) -> io::Result<UsbFs> {
        use nix::fcntl::OFlag;
        let path = format!("/dev/bus/usb/{:03}/{:03}", bus, dev);
        let path = std::path::Path::new(&path);
        let handle = nix::fcntl::open(
            path,
            OFlag::O_RDWR | OFlag::O_NOCTTY | OFlag::O_NONBLOCK,
            nix::sys::stat::Mode::empty(),
        )
        .map_err(|_| io::Error::last_os_error())?;

        let res = UsbFs {
            handle: unsafe { std::fs::File::from_raw_fd(handle) },
            claims: vec![],
            capabilities: 0,
            transfers: Vec::new(),
            descriptors: None,
            bus_dev: (bus, dev),
            read_only: false,
        };

        // res.descriptors();

        Ok(res)
    }

    pub fn reset(&mut self) -> io::Result<()> {
        let res = unsafe { usb_reset(self.handle.as_raw_fd()) };
        match res {
            Err(_) => Err(io::Error::last_os_error()),
            Ok(_) => Ok(()),
        }
    }

    pub fn clear_halt(&mut self, ep: u8) -> io::Result<()> {
        let res = unsafe {
            let mut ep32 = (ep & 0x7f) as u32;
            usb_clear_halt(self.handle.as_raw_fd(), &mut ep32)
        };
        match res {
            Err(_) => Err(io::Error::last_os_error()),
            Ok(_) => Ok(()),
        }
    }

    pub fn handle(&self) -> &std::fs::File {
        &self.handle
    }

    pub fn descriptors(&mut self) -> &Option<UsbDevice> {
        if self.descriptors.is_none() {
            self.descriptors = UsbDevice::from_usbcore(self).ok();
        }

        &self.descriptors
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
    /// // poll then
    /// let urb = usb.async_response()
    /// match transfer {
    ///    Ok(transfer) => { // do stuff }
    ///    Err(e) if e.kind() == ErrorKind::WouldBlock => {},
    ///    Err(e) => { return Err(e); }
    /// }
    /// ```
    /// The returned Transfer can be reused after call transfer.flush()
    pub fn async_response(&mut self) -> io::Result<TransferKind> {
        let urb: *mut UsbFsUrb = ptr::null_mut();
        let urb = unsafe {
            let _ = usb_reapurbndelay(self.handle.as_raw_fd(), &urb)
                .map_err(|_| io::Error::last_os_error())?;
            if urb.is_null() {
                panic!("URB must not be null something is buggy send bug report to usbapi-rs developer");
            }
            Box::from_raw(urb)
        };

        let ep = Endpoint::new(urb.endpoint);
        if ep.is_bulk() {
            Ok(TransferKind::Bulk(bulk_from_urb(*urb)?))
        } else if ep.is_control() {
            Ok(TransferKind::Control(control_from_urb(*urb)?))
        } else {
            Ok(TransferKind::Invalid(ep))
        }
    }

    /// Read all URB responses if there are any pending and store it in transfers
    /// Should be called after mio poll if there is any pending usb_submit's.
    /// The TransferKind is stored in transfers and can be read using
    /// collect_responses()
    ///
    /// Example usage:
    /// ```
    /// usb.submit_bulk(bulk);
    /// poll.poll(&mut events, Duration::from_secs(1))?;
    /// if !events.is_empty() {
    ///     usb.async_response_all()?;
    ///     for transfer in usb.collect_responses() { // Do stuff }
    /// }
    /// ```
    ///
    pub fn async_response_all(&mut self) -> std::io::Result<usize> {
        loop {
            match self.async_response() {
                Ok(transfer) => {
                    self.transfers.push(transfer);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    return Ok(self.transfers.len());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    /// Take all bulk/control responses
    pub fn collect_responses(&mut self) -> Vec<TransferKind> {
        self.transfers.drain(..).collect()
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
        if res == Ok(0) {
            let c_str: &CStr = CStr::from_bytes_with_nul(&driver.driver)
                .map_err(|e| Error::new(ErrorKind::Other, format!("{}", e)))?;
            let name: &str = c_str.to_str().unwrap_or("");
            if name != "usbfs" {
                panic!("FIXME the unload driver API is broken and need to be fixed");
            }
        }
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

    ///
    /// Blocked bulk read
    /// Consider use @async_transfer() instead.
    pub fn bulk_read(&self, ep: u8, mem: &mut [u8], timeout: TimeoutMillis) -> io::Result<u32> {
        self.bulk(
            Endpoint::bulk_in(ep).into(),
            mem.as_mut_ptr() as *mut libc::c_void,
            mem.len() as u32,
            timeout,
        )
    }

    /// Blocked bulk write
    /// consider use @async_transfer() instead
    pub fn bulk_write(&self, ep: u8, mem: &[u8], timeout: TimeoutMillis) -> io::Result<u32> {
        self.bulk(
            ep & 0x7F,
            mem.as_ptr() as *mut libc::c_void,
            mem.len() as u32,
            timeout,
        )
    }

    fn bulk(
        &self,
        ep: u8,
        mem: *mut libc::c_void,
        length: u32,
        timeout: TimeoutMillis,
    ) -> io::Result<u32> {
        let mut bulk = CBulkTransfer {
            ep: ep as u32,
            length,
            timeout_ms: timeout.0,
            data: mem,
        };

        let res = unsafe { usb_bulk_transfer(self.handle.as_raw_fd(), &mut bulk) }
            .map_err(|_| io::Error::last_os_error())?;
        Ok(res as u32)
    }

    /// Get descriptor string with id for default interface
    pub fn get_descriptor_string(&mut self, id: u8) -> std::io::Result<String> {
        self.get_descriptor_string_iface(0, id)
    }

    /// Get descriptor string with id for interface
    pub fn get_descriptor_string_iface(&mut self, iface: u16, id: u8) -> std::io::Result<String> {
        if id == 0 {
            return Err(Error::new(
                ErrorKind::Other,
                "Cannot get descriptor string for zero ID",
            ));
        }
        if self.read_only {
            return Err(Error::new(
                ErrorKind::Other,
                "Can't read descriptors since has been open as ready only",
            ));
        }
        let ctrl = self.new_control_in(
            0x80,               // request_type
            0x06,               // request
            0x0300 | id as u16, // value
            iface,              // index
            256,                // Max read
        )?;
        match self.control_async_wait(ctrl, TimeoutMillis::from(100)) {
            Ok(control) => {
                let data = control.buffer_from_raw();
                let length = data.len();
                if length % 2 != 0 || length <= 2 {
                    log::error!(
                        "Received an odd or short descriptor string of length {} for ID {}",
                        length,
                        id
                    );
                    return Ok("Invalid descriptor".into());
                }
                let n = length / 2;
                let mut x = [0; 2];
                let mut utf16 = Vec::with_capacity(n);
                for i in 1..n {
                    x.copy_from_slice(&data[2 * i..2 * i + 2]);
                    utf16.push(u16::from_le_bytes(x));
                }
                Ok(String::from_utf16_lossy(&utf16))
            }
            Err(e) => Err(Error::new(
                ErrorKind::Other,
                format!("Failed to get descriptor string cause: {}", e),
            )),
        }
    }

    fn mmap(&mut self, length: usize) -> io::Result<*mut u8> {
        let ptr = unsafe {
            let ptr = libc::mmap(
                ptr::null_mut(),
                length as libc::size_t,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                self.handle.as_raw_fd(),
                0,
            );
            if ptr == libc::MAP_FAILED {
                return Err(io::Error::from_raw_os_error(
                    nix::errno::Errno::ENOMEM as i32,
                ));
            }
            ptr
        } as *mut u8;

        Ok(ptr)
    }

    fn munmap(mem: *mut u8, size: usize) {
        unsafe {
            libc::munmap(mem as *mut libc::c_void, size);
        };
    }

    /// Send a async transfer
    /// It is up to the enduser to poll the file descriptor for a result.
    fn submit_urb(&mut self, urb: Box<UsbFsUrb>) -> io::Result<i32> {
        let urb: *mut UsbFsUrb = Box::into_raw(urb);

        let res = unsafe { usb_submit_urb(self.handle.as_raw_fd(), &*urb) }
            .map_err(|_| io::Error::last_os_error())?;
        Ok(res)
    }

    /// Submit a new bulk transfer this will not block.
    /// One shall call mio poll and async_response(_all) after this call to get the transfer back
    /// Note that if the transfer is reused the user must call flush() and fill it with data
    /// before submit_bulk.
    /// Example:
    ///
    /// ```
    /// bulk_out.flush()?;
    /// bulk.write_all(&b"HELLO\n")?;
    /// usb.submit_bulk(bulk);
    /// // poll
    /// resp = usb.async_response();
    /// match resp { // do stuff }
    /// ```
    pub fn submit_bulk(&mut self, bulk: BulkTransfer) -> io::Result<i32> {
        if bulk.actual_length != 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Make sure call flush() before call submit bulk when reuse a transfer.",
            ));
        }
        let urb = Box::new(UsbFsUrb::from(bulk));
        self.submit_urb(urb)
    }

    /// Submit a new control transfer this will not block.
    /// One shall call mio poll and async_response(_all) after this call to get the transfer back
    /// Note that if the transfer is reused the user must call flush() before pass it to
    /// submit_control.
    pub fn submit_control(&mut self, control: ControlTransfer) -> io::Result<i32> {
        if control.actual_length != 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Make sure call flush() before call submit_control when reuse a transfer.",
            ));
        }
        let urb = Box::new(UsbFsUrb::from(control));
        self.submit_urb(urb)
    }

    /// Wait for control response up to timeout ms.
    /// If it find other transfers those are stored in transfers
    /// and can be read using responses()
    pub fn control_async_wait(
        &mut self,
        ctrl: ControlTransfer,
        timeout_ms: TimeoutMillis,
    ) -> io::Result<ControlTransfer> {
        let timeout_ms = timeout_ms.0;
        self.submit_control(ctrl)?;
        let instant = Instant::now();
        loop {
            match self.async_response() {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_micros(1));
                }
                Err(e) => {
                    return Err(e);
                }
                Ok(transfer) => match transfer {
                    TransferKind::Control(control) => {
                        return Ok(control);
                    }
                    _ => self.transfers.push(transfer),
                },
            }
            if instant.elapsed() >= Duration::from_millis(timeout_ms as u64) {
                return Err(Error::new(ErrorKind::TimedOut, ""));
            }
        }
    }
}

impl Drop for UsbFs {
    fn drop(&mut self) {
        for claim in &self.claims {
            if self.release_interface(*claim).is_ok() {};
        }
    }
}
