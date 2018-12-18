mod linux;
#[cfg(target_os="linux")] pub use crate::os::linux::enumerate::UsbEnumerate;
#[cfg(target_os="linux")] pub use crate::os::linux::usbfs::UsbFs as UsbCore;
#[cfg(target_os="linux")] pub use crate::os::linux::usbfs::ControlTransfer;
