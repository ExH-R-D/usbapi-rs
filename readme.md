Let's talk to USB peripherals in RUST.

Very very very not production ready.

The idea is to make a user space driver for Rust in Linux.

Hopefully can replace libusb-rs C binding in the feature.
(At least on Linux.)

I am very new to RUST so this project will probably be rewritten several times....

Don't kill me for doing it wrong however you are free to fork or send pull request and ideas....

TODO

* Add bulk Syncron API
* Make sure claims drop correcly and don't try claim already bound.
* Add async API's
* Add mmap API's
* Split code in different modules since it already ggetting big

....

