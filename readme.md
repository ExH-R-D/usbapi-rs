# Native USB API for Rust

This does *not* depend on libusb and is a raw implementation in rust.

Only Linux is supported atm.

You are free to fork or send pull request and ideas...

## Supported functions

- [X] Enumerate USB peripherals
- [X] Zero copy(mmap) buffers.
- [X] Sync bulk/control API's
- [X] Async Bulk transmittions

## TODO

- [ ] Fix possible leak in sync_respond()
- [ ] Add isochronous support

### For those who use below, feel free to send a pull request:

- [ ] Support freebsd
- [ ] Support netbsd
- [ ] Support haiku
- [ ] Support OSX
- [ ] Support Windows
