





use usbapi::*;
fn main() -> Result<(), std::io::Error> {
    let usb = UsbEnumerate::from_sysfs()?;

    for (_bus_address, device) in usb.devices() {
        if device.device.id_vendor == 0x483 && device.device.id_product == 0x5740 {
            println!("Found one STM32 device. (Note if there  is more than one STM connected to the host the rest will be ignored.");
            let mut usb = UsbCore::from_device(&device).expect("Could not open device");
            println!("Capabilities: 0x{:02X?}", usb.capabilities());
            let _ = usb.claim_interface(0).is_ok();
            println!(
                "Manufacturer: {}",
                usb.get_descriptor_string(device.device.imanufacturer)
                    .unwrap_or("?".into())
            );
            println!(
                "Product: {}",
                usb.get_descriptor_string(device.device.iproduct)
                    .unwrap_or("?".into())
            );
            println!(
                "Serial: {}",
                usb.get_descriptor_string(device.device.iserial)
                    .unwrap_or("?".into())
            );
        }
    }

    println!("Exited successfully");
    Ok(())
}
