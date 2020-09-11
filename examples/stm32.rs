/// This example is ugly
/// For more see: https://gitlab.com/mike7b4/dfuflash
use mio::{Evented, Events, Poll, PollOpt, Ready, Token};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use usbapi::*;
const SYNC_BYTE: u8 = 250;
const BULK_IN: u8 = 0x81;
const BULK_OUT: u8 = 0x1;

fn block_transfer(usb: &mut UsbCore) -> Result<(), std::io::Error> {
    let mut mem: [u8; 64] = [0; 64];
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    println!("Read once to check if there where some garbage");
    println!("1 {} received data {:?}", len, &mem[0..len as usize]);
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    println!("2 {} received data: {:?}", len, &mem[0..len as usize]);
    assert!(len == 0);
    let len = usb.bulk_write(1, "$".to_string().as_bytes()).unwrap_or(0);
    assert!(len == 1);
    println!("1 {} sent data", len);
    let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
    assert!(len > 0);
    println!(
        "3 Received data try stringify: {}",
        String::from_utf8_lossy(&mem[0..len as usize])
    );
    Ok(())
}

fn poll_send(poll: &Poll, usb: &mut UsbCore) -> Result<(), std::io::Error> {
    let mut events = Events::with_capacity(1);
    let mut urbtx = usb.new_bulk(BULK_OUT, 1)?;
    let slice = urbtx.buffer_from_raw_mut();
    slice[0] = SYNC_BYTE;
    println!("=== urbtx before poll ====\n{}", urbtx);
    usb.async_transfer(urbtx)?;
    if poll.poll(&mut events, Some(Duration::from_millis(100)))? == 0 {
        panic!("Poll did not return anything");
    }
    urbtx = usb.async_response()?;
    println!("=== urb tx after poll ===\n{}", urbtx);
    Ok(())
}

fn poll_transfer(terminate: Arc<AtomicBool>, usb: &mut UsbCore) -> Result<(), std::io::Error> {
    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1);
    usb.register(&poll, Token(0), Ready::writable(), PollOpt::edge())?;

    poll_send(&poll, usb)?;
    // send one byte
    let mut urbrx = usb.new_bulk(BULK_IN, 64)?;
    usb.async_transfer(urbrx).unwrap_or(0);
    while !terminate.load(Ordering::Relaxed) {
        poll.poll(&mut events, Some(Duration::from_millis(100)))?;
        for e in &events {
            println!("got event: {:?}", e);
            urbrx = usb.async_response().unwrap();
            println!("=== urb rx after poll ===\n{}", urbrx);
            println!("Got bytes:\n{:02X?}", urbrx.buffer_from_raw());
            //poll_send(&poll, usb);
            usb.async_transfer(urbrx).unwrap_or(0);
        }
    }
    Ok(())
}

fn main() -> Result<(), std::io::Error> {
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&term)).unwrap();

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
            );
            println!(
                "Product: {}",
                usb.get_descriptor_string(device.device.iproduct)
            );
            println!(
                "Serial: {}",
                usb.get_descriptor_string(device.device.iserial)
            );
            let _ = usb.claim_interface(1).is_ok();
            match usb.control(ControlTransfer::new(0x21, 0x22, 0x3, 0, None, 100)) {
                Ok(_) => {}
                Err(err) => println!("Send bytes to control failed {}", err),
            };

            block_transfer(&mut usb)?;

            poll_transfer(term.clone(), &mut usb)?;
            break;
        }
    }

    println!("Exited succefully");
    Ok(())
}
