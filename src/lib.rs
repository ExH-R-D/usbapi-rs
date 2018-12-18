use nix::*;
pub mod descriptors;
pub mod os;
/* Unsure if this is the correct way do it... */
#[cfg(target_os="linux")] pub use crate::os::linux::enumerate::UsbEnumerate;
#[cfg(target_os="linux")] pub use crate::os::linux::usbfs::UsbFs as UsbCore;
#[cfg(target_os="linux")] pub use crate::os::linux::usbfs::ControlTransfer;
