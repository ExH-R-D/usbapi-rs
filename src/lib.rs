pub mod descriptors;
pub mod os;
pub mod usb_transfer;
#[cfg(target_os = "linux")]
pub use os::linux::constants::*;
#[cfg(target_os = "linux")]
pub use os::linux::enumerate::UsbEnumerate;
#[cfg(target_os = "linux")]
pub use os::linux::usb_device::UsbDevice;
#[cfg(target_os = "linux")]
pub use os::linux::usbfs::UsbFs as UsbCore;
pub use usb_transfer::{ControlTransfer, UsbCoreTransfer, UsbTransfer};

#[derive(Debug, Clone)]
pub struct TimeoutMillis(u32);
impl TimeoutMillis {
    fn new(timeout: u32) -> Self {
        Self { 0: timeout }
    }
}

impl From<u32> for TimeoutMillis {
    fn from(timeout_ms: u32) -> Self {
        TimeoutMillis::new(timeout_ms)
    }
}
