extern crate nix;
extern crate signal_hook;
mod os;
mod descriptors;

use os::linuxusbfs::UsbFsDriver;
use os::linuxusbfs::UsbEnumerate;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use nix::*;
use std::path::Path as Path;
use std::thread;
use std::time::Duration;

fn main() {
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&term)).unwrap();

    let mut usb = UsbEnumerate::new();
    usb.enumerate(Path::new("/dev/bus/usb/")).expect("Could not find /dev/bus/usb are you running windows or maybe freebsd or mac or... whatever feel free to add a patch :)");

    let device = usb.get_device_from_bus(3, 5).expect("Could not get device");
    println!("{}", device);

    let mut usb = UsbFsDriver::from_device(&device).expect("FIXME actually cant fail");
    println!("Capabilities: 0x{:02X?}", usb.capabilities().unwrap());
    usb.claim_interface(0).is_ok();
    match usb.control() {
        Ok(_) => {},
        Err(err) => println!("Send bytes to control failed {}", err),
    };

    let mut mem: [u8; 64] = [0; 64];
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    println!("{} data {:?}", len, &mem[0..len as usize]);
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    println!("{} data: {:?}", len, &mem[0..len as usize]);
    let len = usb.bulk_write(1, "$".to_string().as_bytes()).unwrap_or(0);
    println!("{} sent data", len);
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    println!("{} data: {:?}", len, &mem[0..len as usize]);
    loop {
        if term.load(Ordering::Relaxed) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    println!("Drop dead");
}
