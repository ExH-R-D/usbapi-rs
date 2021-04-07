use std::time::Duration;
pub mod descriptors;
pub mod endpoint;
pub mod os;
pub mod usb_transfer;
pub use endpoint::{Endpoint, ENDPOINT_IN, ENDPOINT_OUT};
#[cfg(target_os = "linux")]
pub use os::linux::constants::*;
#[cfg(target_os = "linux")]
pub use os::linux::enumerate::UsbEnumerate;
#[cfg(target_os = "linux")]
pub use os::linux::usb_device::UsbDevice;
#[cfg(target_os = "linux")]
pub use os::linux::usbfs::UsbFs as UsbCore;
pub use usb_transfer::{BufferSlice, BulkTransfer, ControlTransfer, TransferKind, UsbCoreDriver};

#[derive(Debug, Clone)]
pub struct TimeoutMillis(u32);
impl From<u32> for TimeoutMillis {
    fn from(timeout_ms: u32) -> Self {
        Self { 0: timeout_ms }
    }
}

impl From<Duration> for TimeoutMillis {
    fn from(duration: Duration) -> Self {
        Self {
            0: duration.as_millis() as u32,
        }
    }
}
