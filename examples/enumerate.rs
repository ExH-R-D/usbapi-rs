use usbapi::UsbEnumerate;
use usbapi::UsbCore;
use toml;
use serde_json::json;
fn main() {
    let mut usb = UsbEnumerate::new();
    usb.enumerate().expect("Could not find /dev/bus/usb are you running windows or maybe freebsd or mac or... whatever feel free to add a patch :)");
    println!("{}", toml::to_string(usb.devices()).unwrap());
    println!("{}", json!(usb.devices()));
}
