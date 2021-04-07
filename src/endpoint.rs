use std::fmt;
#[derive(Clone, Copy, PartialEq)]
pub struct Endpoint(u8);
#[allow(dead_code)]
pub const ENDPOINT_IN: u8 = 0x80;
#[allow(dead_code)]
pub const ENDPOINT_OUT: u8 = 0x00;

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EP: {} ({})",
            self.0,
            if self.is_bulk_in() {
                "Bulk In"
            } else if self.is_bulk_out() {
                "Bulk Out"
            } else if self.is_control() {
                "Control"
            } else {
                "?TBD?"
            }
        )
    }
}

impl From<Endpoint> for u8 {
    fn from(ep: Endpoint) -> u8 {
        ep.0
    }
}

impl Endpoint {
    pub fn new(ep: u8) -> Self {
        Self { 0: ep }
    }

    pub fn bulk_out(ep: u8) -> Self {
        Self { 0: (ep & 0xF) }
    }

    pub fn bulk_in(ep: u8) -> Self {
        Self {
            0: ENDPOINT_IN | (ep & 0xF),
        }
    }

    pub fn is_control(&self) -> bool {
        self.0 == 0
    }

    pub fn is_bulk_in(&self) -> bool {
        // bulk can not be 0
        self.0 & ENDPOINT_IN == ENDPOINT_IN && self.0 & 0x0F != 0
    }

    pub fn is_bulk_out(&self) -> bool {
        // bulk can not be 0
        self.0 & 0xF0 == 0 && self.0 & 0x0F != 0
    }

    pub fn is_bulk(&self) -> bool {
        self.is_bulk_in() || self.is_bulk_out()
    }
}
