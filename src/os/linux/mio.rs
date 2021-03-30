use super::usbfs::UsbFs;
use mio::event::Source;
use mio::unix::SourceFd;
use mio::{Interest, Poll, Registry, Token};
use std::io;
use std::os::unix::io::AsRawFd;
impl Source for UsbFs {
    fn register(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        SourceFd(&self.handle.as_raw_fd()).register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &Registry,
        token: Token,
        interests: Interest,
    ) -> io::Result<()> {
        SourceFd(&self.handle.as_raw_fd()).reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &Registry) -> io::Result<()> {
        SourceFd(&self.handle.as_raw_fd()).deregister(registry)
    }
}
