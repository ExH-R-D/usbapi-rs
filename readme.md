# Native USB API for Rust

Inspired by the C http://libusb.info driver.

Rust usbapi crate currently only support Linux.

UsbAPI crate does *not* have any dependencies on libusb C API's and is a clean implementation in Rust using ioctl/mmap calls on Linux using nix low level crate.

You are free to fork or send pull request and make it work on other platforms.

# Dependencies

See Cargo.toml I try to use as less as possible.

## Supported functions in Linux

- [X] Enumerate USB peripherals
- [X] Zero copy using mmap buffers.
- [X] Sync bulk API's
- [X] Async bulk and control transmissions
- [X] Transfers are safe and can't be accessed after passed to kernel
- [X] Optional all descriptors can be serialized if feature serde is enabled.
- [X] Optional mio support

## TODO

When I started this project I was new in Rust. Some stuff will change.

 - [ ] Cleanup traits implementations for easier port to other platforms
 - [X] Use valgrind to cleanup possible leaks in unsafe code (eg mmap etc...
 - [ ] Add isochronous support
 - [ ] Add interrupt endpoints
 - [X] Use log crate instead of eprintln and println for debug.
 - [ ] claim_interface will panic if kernel driver is loaded since unload driver is not implemented cant test so not implemented feel free to send patch if needed.

### For those who use any of below platforms, feel free to send a pull request:

 - [ ] Support FreeBSD/OpenBSD
 - [ ] Support NetBSD
 - [ ] Support Haiku
 - [ ] Support OSX
 - [ ] Support Windows
