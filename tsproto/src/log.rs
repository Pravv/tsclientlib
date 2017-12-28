use std::net::SocketAddr;

use futures::{self, Sink, Stream};
use slog::Logger;

use {Error, SinkWrapper, StreamWrapper};
use packets::{Packet, UdpPacket};

pub struct PacketLogger;
impl PacketLogger {
    fn prepare_logger(
        logger: &Logger,
        addr: SocketAddr,
        is_client: bool,
        incoming: bool,
    ) -> Logger {
        let in_s = if incoming {
            "\x1b[1;32mIN\x1b[0m"
        } else {
            "\x1b[1;31mOUT\x1b[0m"
        };
        let to_s = if is_client { "S" } else { "C" };
        let addr_s = format!("{}", addr);
        logger.new(o!("dir" => in_s, "to" => to_s, "addr" => addr_s))
    }

    pub fn log_udp_packet(
        logger: &Logger,
        addr: SocketAddr,
        is_client: bool,
        incoming: bool,
        packet: &UdpPacket,
    ) {
        let logger = Self::prepare_logger(logger, addr, is_client, incoming);
        debug!(logger, "UdpPacket"; "content" => ?::HexSlice(&packet.0));
    }

    pub fn log_packet(
        logger: &Logger,
        addr: SocketAddr,
        is_client: bool,
        incoming: bool,
        packet: &Packet,
    ) {
        let logger = Self::prepare_logger(logger, addr, is_client, incoming);
        debug!(logger, "Packet"; "content" => ?packet);
    }
}

pub struct UdpPacketStreamLogger<
    Inner: Stream<Item = (SocketAddr, UdpPacket), Error = Error>,
> {
    inner: Inner,
    logger: Logger,
    is_client: bool,
}

impl<Inner: Stream<Item = (SocketAddr, UdpPacket), Error = Error>>
    UdpPacketStreamLogger<Inner> {
    pub fn new(
        inner: Inner,
        logger: Logger,
        is_client: bool,
    ) -> Self {
        Self {
            inner,
            logger,
            is_client,
        }
    }
}

impl<Inner: Stream<Item = (SocketAddr, UdpPacket), Error = Error>>
    StreamWrapper<(SocketAddr, UdpPacket), Error, Inner> for
    UdpPacketStreamLogger<Inner> {
    /// (logger, is_client)
    type A = (Logger, bool);

    fn wrap(inner: Inner, (logger, is_client): Self::A) -> Self {
        UdpPacketStreamLogger::new(inner, logger, is_client)
    }
}

impl<Inner: Stream<Item = (SocketAddr, UdpPacket), Error = Error>> Stream
    for UdpPacketStreamLogger<Inner> {
    type Item = (SocketAddr, UdpPacket);
    type Error = Error;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        let res = self.inner.poll();
        if let Ok(futures::Async::Ready(Some((addr, ref packet)))) = res {
            PacketLogger::log_udp_packet(
                &self.logger,
                addr,
                self.is_client,
                true,
                packet,
            );
        }
        res
    }
}

pub struct UdpPacketSinkLogger<
    Inner: Sink<SinkItem = (SocketAddr, UdpPacket), SinkError = Error>,
> {
    inner: Inner,
    logger: Logger,
    is_client: bool,
    /// The buffer to save a packet that is already logged.
    buf: Option<(SocketAddr, UdpPacket)>,
}

impl<Inner: Sink<SinkItem = (SocketAddr, UdpPacket), SinkError = Error>>
    UdpPacketSinkLogger<Inner> {
    pub fn new(
        inner: Inner,
        logger: Logger,
        is_client: bool,
    ) -> Self {
        Self {
            inner,
            logger,
            is_client,
            buf: None,
        }
    }
}

impl<Inner: Sink<SinkItem = (SocketAddr, UdpPacket), SinkError = Error>>
    SinkWrapper<(SocketAddr, UdpPacket), Error, Inner> for
    UdpPacketSinkLogger<Inner> {
    /// (logger, is_client)
    type A = (Logger, bool);

    fn wrap(inner: Inner, (logger, is_client): Self::A) -> Self {
        UdpPacketSinkLogger::new(inner, logger, is_client)
    }
}

impl<Inner: Sink<SinkItem = (SocketAddr, UdpPacket), SinkError = Error>>
    Sink for UdpPacketSinkLogger<Inner> {
    type SinkItem = (SocketAddr, UdpPacket);
    type SinkError = Error;

    fn start_send(
        &mut self,
        (addr, packet): Self::SinkItem,
    ) -> futures::StartSend<Self::SinkItem, Self::SinkError> {
        // Check if the buffer is full
        if let Some(p) = self.buf.take() {
            if let futures::AsyncSink::NotReady(p) = self.inner.start_send(p)? {
                self.buf = Some(p);
                return Ok(futures::AsyncSink::NotReady((addr, packet)));
            }
        }

        PacketLogger::log_udp_packet(
            &self.logger,
            addr,
            self.is_client,
            false,
            &packet,
        );
        let res = self.inner.start_send((addr, packet))?;
        // Buffer the packet if it was not sent
        if let futures::AsyncSink::NotReady(p) = res {
            self.buf = Some(p);
            Ok(futures::AsyncSink::Ready)
        } else {
            Ok(res)
        }
    }

    fn poll_complete(&mut self) -> futures::Poll<(), Self::SinkError> {
        // Check if the buffer is full
        if let Some(p) = self.buf.take() {
            if let futures::AsyncSink::NotReady(p) = self.inner.start_send(p)? {
                self.buf = Some(p);
                return Ok(futures::Async::NotReady);
            }
        }

        self.inner.poll_complete()
    }

    fn close(&mut self) -> futures::Poll<(), Self::SinkError> {
        self.inner.poll_complete()
    }
}

pub struct PacketStreamLogger<
    Inner: Stream<Item = (SocketAddr, Packet), Error = Error>,
> {
    inner: Inner,
    logger: Logger,
    is_client: bool,
}

impl<Inner: Stream<Item = (SocketAddr, Packet), Error = Error>>
    PacketStreamLogger<Inner> {
    pub fn new(
        inner: Inner,
        logger: Logger,
        is_client: bool,
    ) -> Self {
        Self {
            inner,
            logger,
            is_client,
        }
    }
}

impl<Inner: Stream<Item = (SocketAddr, Packet), Error = Error>>
    StreamWrapper<(SocketAddr, Packet), Error, Inner> for
    PacketStreamLogger<Inner> {
    /// (logger, is_client)
    type A = (Logger, bool);

    fn wrap(inner: Inner, (logger, is_client): Self::A) -> Self {
        PacketStreamLogger::new(inner, logger, is_client)
    }
}

impl<Inner: Stream<Item = (SocketAddr, Packet), Error = Error>> Stream
    for PacketStreamLogger<Inner> {
    type Item = (SocketAddr, Packet);
    type Error = Error;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        let res = self.inner.poll();
        if let Ok(futures::Async::Ready(Some((addr, ref packet)))) = res {
            PacketLogger::log_packet(
                &self.logger,
                addr,
                self.is_client,
                true,
                packet,
            );
        }
        res
    }
}

pub struct PacketSinkLogger<
    Inner: Sink<SinkItem = (SocketAddr, Packet), SinkError = Error>,
> {
    inner: Inner,
    logger: Logger,
    is_client: bool,
}

impl<Inner: Sink<SinkItem = (SocketAddr, Packet), SinkError = Error>>
    PacketSinkLogger<Inner> {
    pub fn new(
        inner: Inner,
        logger: Logger,
        is_client: bool,
    ) -> Self {
        Self {
            inner,
            logger,
            is_client,
        }
    }
}

impl<Inner: Sink<SinkItem = (SocketAddr, Packet), SinkError = Error>>
    SinkWrapper<(SocketAddr, Packet), Error, Inner> for PacketSinkLogger<Inner> {
    /// (logger, is_client)
    type A = (Logger, bool);

    fn wrap(inner: Inner, (logger, is_client): Self::A) -> Self {
        PacketSinkLogger::new(inner, logger, is_client)
    }
}

impl<Inner: Sink<SinkItem = (SocketAddr, Packet), SinkError = Error>> Sink
    for PacketSinkLogger<Inner> {
    type SinkItem = (SocketAddr, Packet);
    type SinkError = Error;

    fn start_send(
        &mut self,
        (addr, packet): Self::SinkItem,
    ) -> futures::StartSend<Self::SinkItem, Self::SinkError> {
        PacketLogger::log_packet(
            &self.logger,
            addr,
            self.is_client,
            false,
            &packet,
        );
        self.inner.start_send((addr, packet))
    }
    fn poll_complete(&mut self) -> futures::Poll<(), Self::SinkError> {
        self.inner.poll_complete()
    }
    fn close(&mut self) -> futures::Poll<(), Self::SinkError> {
        self.inner.poll_complete()
    }
}
