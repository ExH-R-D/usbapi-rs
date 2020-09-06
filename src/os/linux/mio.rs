use super::usbfs::UsbFs;
use mio::event::Evented;
use mio::unix::EventedFd;
use mio::{Poll, PollOpt, Ready, Token};
use std::io;
use std::os::unix::io::AsRawFd;
impl Evented for UsbFs {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.handle.as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.handle.as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        EventedFd(&self.handle.as_raw_fd()).deregister(poll)
    }
}
