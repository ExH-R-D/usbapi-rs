# Native USB API for Rust

Heavily inspired by the C http://libusb.info driver.

Rust usbapi crate currently only support Linux.

UsbAPI crate does *not* have any dependencies on libusb C API's and is a clean implementation in Rust using ioctl/mmap calls on Linux using nix low level crate.

You are free to fork or send pull request and make it work on other platforms.

# Dependencies

See Cargo.toml

## Supported functions in Linux

- [X] Enumerate USB peripherals
- [X] Zero copy using mmap buffers.
- [X] Sync bulk/control API's
- [X] Async bulk transmissions
- [X] Descriptors implements serde for easy serializing to JSON, Toml etc...

## TODO

When I started this project I was new in Rust. Some stuff will change.

 - [ ] serde should be optional feature
 - [ ] USBCore should be done as trait(s) for easier porting to other platforms.
 - [X] Fix possible leak in sync_respond()
 - [ ] Add isochronous support
 - [ ] Use log crate instead of eprintln and println for debug.
 - [ ] Some functions prints errors those should be passed as results
 - [ ] claim_interface will panic if kernel driver is loaded since unload driver is not implemented.

### For those who use any of below platforms, feel free to send a pull request:

 - [ ] Support FreeBSD/OpenBSD
 - [ ] Support NetBSD
 - [ ] Support Haiku
 - [ ] Support OSX
 - [ ] Support Windows
