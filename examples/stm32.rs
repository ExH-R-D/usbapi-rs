extern crate mio;
extern crate usbapi;
use usbapi::os::linux::enumerate::Enumerate;
use usbapi::os::linux::usbfs::UsbFs;
use mio::{Events,Ready, Poll, PollOpt, Token, Evented};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
fn main() {
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&term)).unwrap();

    let mut usb = Enumerate::new();
    usb.enumerate(Path::new("/dev/bus/usb/")).expect("Could not find /dev/bus/usb are you running windows or maybe freebsd or mac or... whatever feel free to add a patch :)");

    for device in usb.devices() {
        if device.device.id_vendor == 0x483 && device.device.id_product == 0x5740 {
            let mut usb = UsbFs::from_device(&device).expect("FIXME actually cant fail");
            let poll = Poll::new().unwrap();
            usb.register(&poll,
             Token(0), Ready::all(), PollOpt::edge());

             println!("Capabilities: 0x{:02X?}", usb.capabilities());
            usb.claim_interface(0).is_ok();
            usb.claim_interface(1).is_ok();
            match usb.control() {
                Ok(_) => {}
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
            println!(
                "As string: {}",
                String::from_utf8_lossy(&mem[0..len as usize])
            );

            let urb = usb.new_bulk(0x1, 1).unwrap();
            let slice = unsafe { std::slice::from_raw_parts_mut(urb.buffer, 1) };
            slice[0] = '$' as u8;
            let len = usb.async_transfer(urb).unwrap_or(0);
            println!("{} sent data", len);
            let mut events = Events::with_capacity(16);
            loop {
                poll.poll(&mut events, Some(Duration::from_millis(100)));
                for e in &events {
            //        usb.async_response(e);
            //let len = usb.async_transfer(0x81, 64).unwrap_or(0);
                    println!("event: {:?}", e);
                      let len = usb.bulk_read(1, &mut mem).unwrap_or(1);
                    println!(
                       "As string: {}",
                      String::from_utf8_lossy(&mem[0..len as usize])
                 );
                }
                // TODO setup a thread to talk to STM via http
                if term.load(Ordering::Relaxed) {
                    break;
                }
//                thread::sleep(Duration::from_millis(100));
            }
            break;
        }
    }

    println!("Drop dead");
}
