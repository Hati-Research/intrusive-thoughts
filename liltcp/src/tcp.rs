use core::{future::poll_fn, task::Poll};

use smoltcp::{
    iface::{Context, SocketHandle},
    socket::tcp::{self, ConnectError, RecvError, SendError},
    storage::RingBuffer,
    wire::{IpEndpoint, IpListenEndpoint},
};

use crate::stack::Stack;

pub struct TcpClient<'a> {
    pub stack: Stack<'a>,
    pub handle: SocketHandle,
}

impl<'a> TcpClient<'a> {
    pub fn new(stack: Stack<'a>, rx_buffer: &'a mut [u8], tx_buffer: &'a mut [u8]) -> Self {
        let rx_buffer = RingBuffer::new(rx_buffer);
        let tx_buffer = RingBuffer::new(tx_buffer);

        let socket = smoltcp::socket::tcp::Socket::new(rx_buffer, tx_buffer);
        let handle = stack.with(|(sockets, _interface)| sockets.add(socket));

        Self { stack, handle }
    }

    fn with<F, U>(&mut self, f: F) -> U
    where
        F: FnOnce(&mut tcp::Socket, &mut Context) -> U,
    {
        self.stack.with(|(sockets, interface)| {
            let mut socket = sockets.get_mut(self.handle);

            f(&mut socket, interface.context())
        })
    }

    pub async fn connect(
        &mut self,
        remote_endpoint: impl Into<IpEndpoint>,
        local_endpoint: impl Into<IpListenEndpoint>,
    ) -> Result<(), ConnectError> {
        self.with(|socket, context| socket.connect(context, remote_endpoint, local_endpoint))?;

        poll_fn(|cx| {
            self.with(|socket, _context| {
                // shamelessly copied from embassy
                match socket.state() {
                    tcp::State::Closed | tcp::State::TimeWait => {
                        Poll::Ready(Err(ConnectError::InvalidState))
                    }
                    tcp::State::Listen => unreachable!(), // marks invalid state
                    tcp::State::SynSent | tcp::State::SynReceived => {
                        socket.register_send_waker(cx.waker());
                        socket.register_recv_waker(cx.waker());
                        Poll::Pending
                    }
                    _ => Poll::Ready(Ok(())),
                }
            })
        })
        .await
    }

    pub async fn send(&mut self, buf: &[u8]) -> Result<usize, SendError> {
        poll_fn(|cx| {
            self.with(|socket, _context| match socket.send_slice(buf) {
                Ok(0) => {
                    socket.register_send_waker(cx.waker());
                    Poll::Pending
                }
                Ok(n) => Poll::Ready(Ok(n)),
                Err(e) => Poll::Ready(Err(e)),
            })
        })
        .await
    }

    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, RecvError> {
        poll_fn(|cx| {
            self.with(|socket, _context| match socket.recv_slice(buf) {
                // return 0 doesn't mean EOF when buf is empty
                Ok(0) if buf.is_empty() => Poll::Ready(Ok(0)),
                Ok(0) => {
                    socket.register_recv_waker(cx.waker());
                    Poll::Pending
                }
                Ok(n) => Poll::Ready(Ok(n)),
                // EOF
                Err(RecvError::Finished) => Poll::Ready(Ok(0)),
                Err(RecvError::InvalidState) => Poll::Ready(Err(RecvError::InvalidState)),
            })
        })
        .await
    }
}
