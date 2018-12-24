use usbapi::UsbEnumerate;
use usbapi::UsbCore;
use std::path::Path as Path;
fn main() {
    let mut usb = UsbEnumerate::new();
    usb.enumerate().expect("Could not find /dev/bus/usb are you running windows or maybe freebsd or mac or... whatever feel free to add a patch :)");
    for device in usb.devices() {
        let mut usb = UsbCore::from_device(&device);
        match usb {
            Ok(mut usb) => {
                println!("Manufacturer: {}", usb.get_descriptor_string(device.device.imanufacturer));
                println!("Product: {}", usb.get_descriptor_string(device.device.iproduct));
                println!("Serial: {}", usb.get_descriptor_string(device.device.iserial_number));
            },
            Err(err) => { println!("{}", err) }
        }
        println!("{}", serde_json::to_string(&device).unwrap());
    }
}
