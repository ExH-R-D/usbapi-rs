use crate::endpoint::*;
use std::fmt;
use std::io;
use std::io::Write;
use std::io::{Error, ErrorKind};

type Deallocate = Box<dyn Fn(*mut u8, usize) + 'static>;

pub struct ControlTransfer {
    pub buffer: *mut u8,
    // buffer length is the value sent to kernel
    pub(crate) buffer_length: u16,
    // capacity of the buffer needed when call deallocator
    pub(crate) buffer_capacity: u16,
    // given back from kernel
    pub(crate) actual_length: u16,
    deallocate: Deallocate,
}

impl fmt::Display for ControlTransfer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "|Control|{}|{}|{}|",
            self.buffer_length, self.actual_length, self.buffer_capacity
        )
    }
}

impl ControlTransfer {
    pub(crate) fn new<DEALOC>(
        buffer: *mut u8,
        buffer_capacity: u16,
        buffer_length: u16,
        deallocate: DEALOC,
    ) -> Self
    where
        DEALOC: Fn(*mut u8, usize) + 'static,
    {
        Self {
            buffer,
            buffer_length,
            buffer_capacity,
            actual_length: 0,
            deallocate: Box::new(deallocate),
        }
    }
}

impl Drop for ControlTransfer {
    fn drop(&mut self) {
        if !self.buffer.is_null() {
            (self.deallocate)(self.buffer, self.buffer_capacity as usize);
        }
    }
}

impl Write for ControlTransfer {
    fn write(&mut self, inbuf: &[u8]) -> io::Result<usize> {
        let buf = unsafe {
            let buf = self.buffer;
            std::slice::from_raw_parts_mut(
                buf.offset(self.buffer_length as isize),
                self.buffer_capacity as usize - self.buffer_length as usize,
            )
        };

        for (i, byte) in inbuf.iter().enumerate() {
            buf[i] = *byte;
            self.buffer_length += 1;
        }
        Ok(inbuf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.actual_length = 0;
        Ok(())
    }
}

pub struct BulkTransfer {
    pub(crate) buffer: *mut u8,
    // Lower layer write or read length
    pub buffer_length: usize,
    pub actual_length: usize,
    pub status: i32,
    // allocedata size
    pub buffer_capacity: usize,
    pub endpoint: Endpoint,
    // give back allocated memory
    deallocate: Deallocate,
}

impl fmt::Display for BulkTransfer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "|{}|{}|{}|{}|",
            self.endpoint, self.buffer_length, self.actual_length, self.buffer_capacity
        )
    }
}

impl BulkTransfer {
    /// Create new bulk input (Read)
    /// the deallocate is called when BulkTransfer goes out of scope to cleanup
    /// allocated memory
    pub fn input<DEALOC>(
        ep: u8,
        buffer: *mut u8,
        buffer_capacity: usize,
        deallocate: DEALOC,
    ) -> Self
    where
        DEALOC: Fn(*mut u8, usize) + 'static,
    {
        Self {
            buffer,
            buffer_capacity,
            buffer_length: buffer_capacity,
            actual_length: 0,
            status: 0,
            endpoint: Endpoint::bulk_in(ep),
            deallocate: Box::new(deallocate),
        }
    }

    /// Create a write
    pub fn output<DEALOC>(
        ep: u8,
        buffer: *mut u8,
        buffer_capacity: usize,
        deallocate: DEALOC,
    ) -> Self
    where
        DEALOC: Fn(*mut u8, usize) + 'static,
    {
        Self {
            buffer,
            buffer_capacity,
            buffer_length: 0, // incremented when we put data in the buffer
            actual_length: 0,
            status: 0,
            endpoint: Endpoint::bulk_out(ep),
            deallocate: Box::new(deallocate),
        }
    }
}

impl Drop for BulkTransfer {
    fn drop(&mut self) {
        if !self.buffer.is_null() {
            (self.deallocate)(self.buffer, self.buffer_capacity);
        }
    }
}

impl Write for BulkTransfer {
    fn write(&mut self, inbuf: &[u8]) -> io::Result<usize> {
        if self.endpoint.is_bulk_in() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Can not write to an bulk in endpoint.",
            ));
        }
        let buf = unsafe {
            let buf = self.buffer;
            std::slice::from_raw_parts_mut(
                buf.offset(self.buffer_length as isize),
                self.buffer_capacity - self.buffer_length,
            )
        };
        assert!(!buf.is_empty());

        for (i, byte) in inbuf.iter().enumerate() {
            buf[i] = *byte;
            self.buffer_length += 1;
        }

        Ok(inbuf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.actual_length = 0;
        if self.endpoint.is_bulk_out() {
            self.buffer_length = 0;
        }
        Ok(())
    }
}

pub enum TransferKind {
    Control(ControlTransfer),
    Bulk(BulkTransfer),
    Invalid(Endpoint),
}

impl fmt::Display for TransferKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TransferKind::Control(control) => write!(f, "Control Transfer: {}", control),
            TransferKind::Bulk(bulk) => write!(f, "Control Transfer: {}", bulk),
            TransferKind::Invalid(ep) => write!(f, "Invalid {}", ep),
        }
    }
}

pub trait BufferSlice {
    fn buffer_from_raw<'a>(&self) -> &'a [u8];
}

pub trait UsbCoreDriver {
    fn new_bulk_in(&mut self, ep: u8, read_capacity: usize) -> io::Result<BulkTransfer>;
    fn new_bulk_out(&mut self, ep: u8, capacity: usize) -> io::Result<BulkTransfer>;

    // Create a new control
    fn new_control(
        &mut self,
        request_type: u8, // bRequestType
        request: u8,      // bRequest
        value: u16,       // wValue
        index: u16,       // wIndex
        length: u16,      // wLength
    ) -> io::Result<ControlTransfer>;

    fn new_control_out(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        buffer: &[u8],
    ) -> io::Result<ControlTransfer> {
        let mut ctrl = self.new_control(
            request_type | ENDPOINT_OUT,
            request,
            value,
            index,
            buffer.len() as u16, // wLength
        )?;
        ctrl.write_all(buffer)?;
        Ok(ctrl)
    }

    fn new_control_in(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        length: u16, // wLength (Read length)
    ) -> io::Result<ControlTransfer> {
        let mut ctrl =
            self.new_control(request_type | ENDPOINT_IN, request, value, index, length)?;
        ctrl.buffer_length += length;
        Ok(ctrl)
    }

    // Create a new control
    fn new_control_nodata(
        &mut self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
    ) -> io::Result<ControlTransfer> {
        self.new_control(request_type, request, value, index, 0)
    }
}
