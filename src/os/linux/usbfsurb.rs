use super::constants::*;
use crate::endpoint::Endpoint;
use crate::usb_transfer::{BufferSlice, BulkTransfer, ControlTransfer};
use std::fmt;
use std::io;
use std::io::{Error, ErrorKind};
#[derive(Debug)]
#[repr(C)]
pub struct UsbFsUrb {
    typ: u8,
    pub endpoint: u8,
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
    pub(crate) usercontext: *mut u8,
}

impl UsbFsUrb {
    pub fn new(typ: u8, ep: Endpoint, ptr: *mut u8, length: usize, user: *mut u8) -> Self {
        UsbFsUrb {
            typ,
            endpoint: ep.into(),
            status: 0,
            flags: 0,
            buffer: ptr,
            buffer_length: length as i32,
            actual_length: 0,
            start_frame: 0,
            stream_id: 0,
            error_count: 0,
            signr: 0,
            usercontext: user,
        }
    }

    /// set buffer_length to length and resets actual_length to 0
    pub fn set_length(&mut self, length: usize) {
        self.actual_length = 0;
        self.buffer_length = length as i32;
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

impl From<BulkTransfer> for UsbFsUrb {
    fn from(bulk: BulkTransfer) -> Self {
        // Box make sure BulkTransfer is not deallocated
        // when when convert to raw below.
        let bulk = Box::new(bulk);
        UsbFsUrb::new(
            USBFS_URB_TYPE_BULK,
            bulk.endpoint,
            bulk.buffer,
            bulk.buffer_length,
            // BulkTransfer is may in theory leak if not passed to kernel
            // to used space.
            Box::into_raw(bulk) as *mut u8,
        )
    }
}

impl From<ControlTransfer> for UsbFsUrb {
    fn from(ctrl: ControlTransfer) -> Self {
        // prevent it from getting freed
        let ctrl = Box::new(ctrl);
        UsbFsUrb::new(
            USBFS_URB_TYPE_CONTROL,
            Endpoint::new(0),
            ctrl.buffer,
            ctrl.buffer_length as usize,
            // this is now "leaked" until transfered to kernel and back
            // to used space. using Box::from_raw on control_from_urb
            Box::into_raw(ctrl) as *mut u8,
        )
    }
}

/// Transfer back urb.usercontext to BulkTransfer
pub(crate) fn bulk_from_urb(urb: UsbFsUrb) -> io::Result<BulkTransfer> {
    let ep = Endpoint::new(urb.endpoint);
    if !ep.is_bulk() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid URB Not Bulk warning possibly leaking userdata",
        ));
    }
    if urb.usercontext.is_null() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid URB usercontext is NULL?",
        ));
    }
    let mut bulk = *unsafe { Box::from_raw(urb.usercontext as *mut BulkTransfer) };
    if u8::from(bulk.endpoint) != urb.endpoint {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Endpoint not match corrupt URB?",
        ));
    }
    // Safe checks before pass it on to user.
    // if buffer_length in urb differ from the TransferBuffer something is smoking
    // Those should panic cause if thi happens something is really wrong
    // in the usbapi-rs and we beter fix those errors.
    assert!(urb.buffer_length as usize == bulk.buffer_length);
    bulk.actual_length = urb.actual_length as usize;
    // if actual is bigger than we asked kernel to store something is smoking
    assert!(bulk.actual_length <= bulk.buffer_length);
    Ok(bulk)
}

/// Transfer back urb.usercontext to BulkTransfer
pub(crate) fn control_from_urb(urb: UsbFsUrb) -> io::Result<ControlTransfer> {
    let ep = Endpoint::new(urb.endpoint);
    if !ep.is_control() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid URB Not not control contextdata is leaked",
        ));
    }
    if urb.usercontext.is_null() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid URB usercontext is NULL?",
        ));
    }
    let mut control = *unsafe { Box::from_raw(urb.usercontext as *mut ControlTransfer) };
    assert!(urb.buffer_length as u16 == control.buffer_length);
    control.actual_length = urb.actual_length as u16;
    assert!(control.actual_length <= control.buffer_length);
    Ok(control)
}

impl BufferSlice for BulkTransfer {
    /// return buffer slice from raw pointer
    /// panic in case actual_length or buffer_capacity is incorrect
    /// Should not happen but if it does we should fix the broken code.
    /// return empty slice if length is 0
    fn buffer_from_raw<'a>(&self) -> &'a [u8] {
        if (self.endpoint.is_bulk_in() && self.actual_length == 0)
            || (self.endpoint.is_bulk_out() && self.buffer_length == 0)
        {
            return &[];
        }
        assert!(self.actual_length <= self.buffer_length);
        assert!(self.buffer_length <= self.buffer_capacity);
        if self.endpoint.is_bulk_in() {
            unsafe { std::slice::from_raw_parts(self.buffer, self.actual_length as usize) }
        } else {
            unsafe { std::slice::from_raw_parts(self.buffer, self.buffer_length as usize) }
        }
    }
}

impl BufferSlice for ControlTransfer {
    fn buffer_from_raw<'a>(&self) -> &'a [u8] {
        if self.actual_length == 0 {
            return &[];
        }
        assert!(self.actual_length <= self.buffer_length - 8);
        unsafe { std::slice::from_raw_parts(self.buffer.offset(8), self.actual_length as usize) }
    }
}
