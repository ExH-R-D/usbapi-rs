use mio::{Events, Interest, Poll, Token};
/// This example is ugly
/// For more see: https://gitlab.com/mike7b4/dfuflash
#[cfg(feature = "mio")]
use std::io::ErrorKind;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use usbapi::endpoint::Endpoint;
use usbapi::*;
const SYNC_BYTE: u8 = 250;
const BULK_IN: u8 = 0x81;
const BULK_OUT: u8 = 0x1;

fn block_transfer(usb: &mut UsbCore) -> Result<(), std::io::Error> {
    let mut mem: [u8; 64] = [0; 64];
    let len = usb.bulk_write(1, &[SYNC_BYTE], 1.into()).unwrap_or(0);
    let len = 0;
    println!("Read once to check if there where some garbage");
    println!("1 {} received data {:?}", len, &mem[0..len as usize]);
    let mut mem: [u8; 64] = [0; 64];
    let len = usb
        .bulk_read(1, &mut mem, TimeoutMillis::from(1))
        .unwrap_or(0);
    println!("2 {} received data: {:?}", len, &mem[0..len as usize]);
    let len = usb
        .bulk_write(1, "$".to_string().as_bytes(), 1.into())
        .unwrap_or(0);
    println!("1 {} sent data", len);
    let mut mem: [u8; 64] = [0; 64];
    let len = usb
        .bulk_read(1, &mut mem, TimeoutMillis::from(1))
        .unwrap_or(0);
    //assert!(len > 0);
    println!(
        "3 Received data try stringify: {}",
        String::from_utf8_lossy(&mem[0..len as usize])
    );
    Ok(())
}

/// Handle async response
fn handle_response(
    mut transfer: TransferKind,
    bulk_tx: &mut Option<BulkTransfer>,
    bulk_rx: &mut Option<BulkTransfer>,
    send_again: &mut bool,
) {
    match transfer {
        TransferKind::Control(mut control) => {
            log::info!("{}", control);
            control.flush();
        }
        TransferKind::Bulk(mut bulk) => {
            if bulk.endpoint.is_bulk_out() {
                assert!(bulk.actual_length == bulk.buffer_length);
                *bulk_tx = Some(bulk);
            } else {
                assert!(bulk.endpoint == Endpoint::new(0x81));
                let raw = bulk.buffer_from_raw();
                let s: Vec<u8> = raw.iter().cloned().filter(|c| *c < 127).collect();
                print!("{}", String::from_utf8_lossy(s.as_slice()).to_string());
                if raw[0] == SYNC_BYTE
                    || s.ends_with(&[0x0A, 0x2E])
                    || s.ends_with(&[0x0A, 0x2C])
                    || (raw[0] == 0x2E && s.len() == 1)
                    || (raw[0] == 0x2C && s.len() == 1)
                {
                    *send_again = true;
                }
                bulk.flush();
                *bulk_rx = Some(bulk);
            }
        }
        TransferKind::Invalid(ep) => {
            log::error!("Invalid endpoint {}", ep);
        }
    }
}

#[cfg(feature = "mio")]
fn poll_transfer(terminate: Arc<AtomicBool>, usb: &mut UsbCore) -> Result<(), std::io::Error> {
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(64);
    poll.registry()
        .register(usb, Token(0), Interest::WRITABLE)?;

    let mut bulk_rx = Some(usb.new_bulk_in(1, 64)?);
    let mut bulk_tx = Some(usb.new_bulk_out(1, 64)?);
    if let Some(bulk_rx) = bulk_rx.take() {
        usb.submit_bulk(bulk_rx)?;
    }
    if let Some(mut bulk_tx) = bulk_tx.take() {
        bulk_tx.write_all(&[SYNC_BYTE, 0xC8])?;
        // submit what we wrote
        usb.submit_bulk(bulk_tx)?;
    }
    let mut send_again = false;
    let mut every = 0;
    let end = Instant::now();
    while !terminate.load(Ordering::Relaxed) && end.elapsed() < Duration::from_secs(10) {
        if let Some(bulk_rx) = bulk_rx.take() {
            usb.submit_bulk(bulk_rx)?;
        }

        let instant = Instant::now();
        poll.poll(&mut events, Some(Duration::from_millis(10)))?;
        if !events.is_empty() && (every % 10) == 0 {
            println!("{:?}", instant.elapsed());
        }
        every += 1;
        if events.is_empty() {
            if send_again {
                if let Some(mut bulk_tx) = bulk_tx.take() {
                    send_again = false;
                    bulk_tx.flush();
                    bulk_tx.write_all(&[0x3F, 0x0A])?;
                    usb.submit_bulk(bulk_tx)?;
                }
            }
        }
        for _ in &events {
            usb.async_response_all()?;
            for transfer in usb.collect_responses() {
                handle_response(transfer, &mut bulk_tx, &mut bulk_rx, &mut send_again);
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), std::io::Error> {
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::SIGQUIT, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&term)).unwrap();
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&term)).unwrap();
    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Info);

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
            let _ = usb.claim_interface(1).is_ok();
            let ctrl = usb.new_control_nodata(0x21, 0x22, 0x3, 0)?;
            match usb.control_async_wait(ctrl, TimeoutMillis::from(100)) {
                Ok(control) => println!("{}", control),
                Err(err) => println!("Send bytes to control failed {}", err),
            };

            block_transfer(&mut usb)?;

            #[cfg(feature = "mio")]
            poll_transfer(term.clone(), &mut usb)?;
            break;
        }
    }

    println!("Exited successfully");
    Ok(())
}
