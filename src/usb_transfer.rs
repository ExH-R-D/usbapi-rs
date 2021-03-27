use std::io;
use std::time::Duration;
#[derive(Clone, Debug)]
pub struct ControlTransfer {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub buffer_length: u16,
    pub timeout: Duration,
    pub buffer: Option<Vec<u8>>,
}

impl ControlTransfer {
    pub fn new_nodata(
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        timeout: Duration,
    ) -> Self {
        ControlTransfer {
            request_type,
            request,
            value,
            index,
            buffer_length: 0,
            buffer: None,
            timeout,
        }
    }

    pub fn new_read(
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        buffer_length: u16,
        timeout: Duration,
    ) -> Self {
        ControlTransfer {
            request_type,
            request,
            value,
            index,
            buffer_length,
            buffer: None,
            timeout,
        }
    }

    pub fn new_with_data(
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        v: Vec<u8>,
        timeout: Duration,
    ) -> Self {
        ControlTransfer {
            request_type,
            request,
            value,
            index,
            buffer_length: v.len() as u16,
            buffer: Some(v),
            timeout,
        }
    }
}

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
