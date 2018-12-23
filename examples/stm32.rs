use usbapi::*;
use mio::{Events,Ready, Poll, PollOpt, Token, Evented};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
fn main() -> Result<(), std::io::Error> {
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&term)).unwrap();

    let mut usb = UsbEnumerate::new();
    usb.enumerate().expect("Could not find /dev/bus/usb are you running windows or maybe freebsd or mac or... whatever feel free to add a patch :)");

    for device in usb.devices() {
        if device.device.id_vendor == 0x483 && device.device.id_product == 0x5740 {
            let mut usb = UsbCore::from_device(&device).expect("Could not open device");
            let poll = Poll::new().unwrap();
            usb.register(&poll, Token(0), Ready::writable(), PollOpt::edge())?;

            println!("Capabilities: 0x{:02X?}", usb.capabilities());
            usb.claim_interface(0).is_ok();
            println!("Manufacturer: {}", usb.get_descriptor_string(device.device.imanufacturer));
            println!("Product: {}", usb.get_descriptor_string(device.device.iproduct));
            println!("Serial: {}", usb.get_descriptor_string(device.device.iserial_number));
            usb.claim_interface(1).is_ok();
            match usb.control(ControlTransfer::new(0x21, 0x22, 0x3, 0, vec!(), 100)) {
                Ok(_) => {}
                Err(err) => println!("Send bytes to control failed {}", err),
            };

            let mut mem: [u8; 64] = [0; 64];
            let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
            println!("1 {} data {:?}", len, &mem[0..len as usize]);
            assert!(len > 0);
            let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
            println!("2 {} data: {:?}", len, &mem[0..len as usize]);
            assert!(len == 0);
            let len = usb.bulk_write(1, "$".to_string().as_bytes()).unwrap_or(0);
            assert!(len == 1);
            println!("1 {} sent data", len);
            let len = usb.bulk_read(1, &mut mem).unwrap_or(0);
            assert!(len > 0);
            println!(
                "3 As string: {}",
                String::from_utf8_lossy(&mem[0..len as usize])
            );

            let urb = usb.new_bulk(0x1, 1).unwrap();
            let slice = urb.buffer_from_raw_mut();
            println!("New URB {}", urb);
            slice[0] = '$' as u8;
            let len = usb.async_transfer(urb).unwrap_or(0);
            // Async return 0 in case of success
            println!("length {}", len);
            assert!(len == 0);
            println!("2 {} sent data", len);
            let mut events = Events::with_capacity(16);
            let mut run = true;
            while run {
                poll.poll(&mut events, Some(Duration::from_millis(100))).unwrap_or(0);
                for e in &events {
                    println!("eventif: {:?}", e);
                    let urb = usb.async_response().unwrap();
                    println!("{}", urb);
                    // Read
                    let rxurb = usb.new_bulk(0x81, 64).unwrap();
                    usb.async_transfer(rxurb).unwrap_or(0);
                    run = false;
                }
                // TODO setup a thread to talk to STM via http
                if term.load(Ordering::Relaxed) {
                    break;
                }
//                thread::sleep(Duration::from_millis(100));
            }
            loop {
                poll.poll(&mut events, Some(Duration::from_millis(100)))?;
                for e in &events {
                    let resp = usb.async_response().unwrap();
                    println!("Urb Response on a read {}", resp);
                    let slice = resp.buffer_from_raw();
                    println!("got string: {}", String::from_utf8_lossy(&slice));
                    let rxurb = usb.new_bulk(0x81, 64).unwrap();
                    usb.async_transfer(rxurb).unwrap_or(0);
                 }
                if term.load(Ordering::Relaxed) {
                    break;
                }
            }
            break;
        }
    }

    println!("Drop dead");
    Ok(())
}
