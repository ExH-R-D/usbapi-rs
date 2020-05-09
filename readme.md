# Native USB API for Rust

Heavily inspired by the C http://libusb.info driver.

Rust usbapi crate only support Linux ATM.

UsbAPI crate does *not* have any dependies on libusb C API's and is a clean implementation in Rust using ioctl/mmap calls on Linux using nix low level crate.

You are free to fork or send pull request and make it work on any of the below platforms.

# Dependies

* serde
* serde-hex
* nix/libc
* mio

# dev-dependies

Examples uses:

* signal-hook
* serde_json
* toml


## Supported functions in Linux

- [X] Enumerate USB peripherals
- [X] Zero copy using mmap buffers.
- [X] Sync bulk/control API's
- [X] Async bulk transmissions
- [X] Descriptors implements serde for easy serializing to JSON, Toml etc...

## TODO

When I started this project I was new in Rust. Some stuff need to be done better:

- [ ] USBCore should be done as trait(s) for easier porting to other platforms.
- [ ] Fix possible leak in sync_respond()
- [ ] Add isochronous support
- [ ] Use log crate instead of eprintln and println for debug.

### For those who use any of below platforms, feel free to send a pull request:

- [ ] Support FreeBSD/OpenBSD
- [ ] Support NetBSD
- [ ] Support Haiku
- [ ] Support OSX
- [ ] Support Windows
