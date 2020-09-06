pub mod descriptors;
pub mod os;
#[cfg(target_os = "linux")]
pub use os::linux::constants::*;
#[cfg(target_os = "linux")]
pub use os::linux::enumerate::UsbEnumerate;
#[cfg(target_os = "linux")]
pub use os::linux::usb_device::UsbDevice;
#[cfg(target_os = "linux")]
pub use os::linux::usbfs::ControlTransfer;
#[cfg(target_os = "linux")]
pub use os::linux::usbfs::UsbFs as UsbCore;
#[cfg(target_os = "linux")]
pub use os::linux::usbfs::{UsbCoreTransfer, UsbTransfer};
