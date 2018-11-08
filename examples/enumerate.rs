extern crate usbapi;
use usbapi::os::linux::enumerate::Enumerate;
use std::path::Path as Path;
fn main() {
    let mut usb = Enumerate::new();
    usb.enumerate(Path::new("/dev/bus/usb/")).expect("Could not find /dev/bus/usb are you running windows or maybe freebsd or mac or... whatever feel free to add a patch :)");
    for device in usb.devices() {
        println!("{}", device);
    }
}
