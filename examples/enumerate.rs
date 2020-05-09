use serde_json;
use toml;
use usbapi::UsbEnumerate;
fn main() {
    let mut usb = UsbEnumerate::new();
    usb.enumerate().expect("Could not find /dev/bus/usb are you running windows or maybe freebsd or mac or... whatever feel free to add a patch :)");
    println!("{}", toml::to_string_pretty(usb.devices()).unwrap());
    println!("{}", serde_json::to_string_pretty(usb.devices()).unwrap());
}
