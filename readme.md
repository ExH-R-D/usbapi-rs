#USBAPI for RUST

The idea is to make a user space driver for Rust in Linux.

Hopefully can replace libusb-rs C binding in the future.

I am very new to RUST so this project will probably be rewritten several times....

Don't kill me for doing it wrong however you are free to fork or send pull request and ideas....

## Supported functions

- [X] Enumerate USB peripherals
- [X] Zero copy(mmap) buffers.
- [X] Sync bulk/control API's
- [X] Async Bulk transmittions

## TODO

- [ ] Fix possible leak in sync_respond()

### For thos who use below, feel free to send a pull request:

- [ ] Support freebsd
- [ ] Support netbsd
- [ ] Support haiku
- [ ] Support OSX
- [ ] Support Windows
