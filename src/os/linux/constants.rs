#[allow(dead_code)]
pub(crate) const USBFS_CAP_ZERO_PACKET: u8 = 0x01;
#[allow(dead_code)]
pub(crate) const USBFS_CAP_BULK_CONTINUATION: u8 = 0x02;
#[allow(dead_code)]
pub(crate) const USBFS_CAP_NO_PACKET_SIZE_LIM: u8 = 0x04;
#[allow(dead_code)]
pub(crate) const USBFS_CAP_BULK_SCATTER_GATHER: u8 = 0x08;
#[allow(dead_code)]
pub(crate) const USBFS_CAP_REAP_AFTER_DISCONNECT: u8 = 0x10;
#[allow(dead_code)]
pub(crate) const USBFS_CAP_MMAP: u8 = 0x20;
#[allow(dead_code)]
pub(crate) const USBFS_CAP_DROP_PRIVILEGES: u8 = 0x40;

#[allow(dead_code)]
pub(crate) const USBFS_URB_TYPE_ISO: u8 = 0;
#[allow(dead_code)]
pub(crate) const USBFS_URB_TYPE_INTERRUPT: u8 = 1;
pub(crate) const USBFS_URB_TYPE_CONTROL: u8 = 2;
pub(crate) const USBFS_URB_TYPE_BULK: u8 = 3;

#[allow(dead_code)]
pub(crate) const USBFS_URB_FLAGS_SHORT_NOT_OK: u32 = 0x01;
#[allow(dead_code)]
pub(crate) const USBFS_URB_FLAGS_ISO_ASAP: u32 = 0x02;
#[allow(dead_code)]
pub(crate) const USBFS_URB_FLAGS_BULK_CONTINUATION: u32 = 0x04;
#[allow(dead_code)]
pub(crate) const USBFS_URB_FLAGS_ZERO_PACKET: u32 = 0x40;
#[allow(dead_code)]
pub(crate) const USBFS_URB_FLAGS_NO_INTERRUPT: u32 = 0x80;

#[allow(dead_code)]
pub const RECIPIENT_DEVICE: u8 = 0x00;
#[allow(dead_code)]
pub const RECIPIENT_INTERFACE: u8 = 0x01;
#[allow(dead_code)]
pub const RECIPIENT_ENDPOINT: u8 = 0x02;
#[allow(dead_code)]
pub const RECIPIENT_OTHER: u8 = 0x03;
#[allow(dead_code)]
pub const REQUEST_TYPE_STANDARD: u8 = 0x00 << 5;
#[allow(dead_code)]
pub const REQUEST_TYPE_CLASS: u8 = 0x01 << 5;
