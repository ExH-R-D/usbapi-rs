extern crate nix;
extern crate mio;
use nix::*;
pub mod descriptors;
pub mod os;
/* Unsure if this is the correct way do it... */
#[cfg(target_os="linux")] pub use os::linux::enumerate::UsbEnumerate;
#[cfg(target_os="linux")] pub use os::linux::usbfs::UsbFs as UsbCore;
#[cfg(target_os="linux")] pub use os::linux::usbfs::ControlTransfer;

