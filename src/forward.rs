use crate::newtypes::SocketAddr;
use async_udp_stream::{AddressedUdp, UdpSocket};
use evmap::{self, ReadHandle, WriteHandle};
use futures::{SinkExt, StreamExt};
pub struct Forwarder {
    socket: UdpSocket,
    mapping: ReadHandle<SocketAddr, SocketAddr>,
}

impl Forwarder {
    pub fn new(socket: UdpSocket) -> (Self, WriteHandle<SocketAddr, SocketAddr>) {
        let (reader, writer) = evmap::new();
        (
            Self {
                socket,
                mapping: reader,
            },
            writer,
        )
    }
    pub async fn forward(mut self) -> ! {
        loop {
            while let Some(Ok(packet)) = self.socket.next().await {
                if let Some(packet) = self.readdress(packet) {
                    if let Err(e) = self.socket.send(packet).await {
                        todo!()
                    }
                }
            }
        }
    }
    fn readdress(&mut self, mut incoming: AddressedUdp) -> Option<AddressedUdp> {
        match self.mapping.get_one(&incoming.address.into()) {
            Some(destination) => {
                incoming.address = (*destination).into();
                Some(incoming)
            }
            None => None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use async_std::test;
    use async_udp_stream::{AddressedUdp, UdpSocket};
    use futures::{
        future::{AbortHandle, Abortable, Aborted},
        join, stream, StreamExt,
    };

    #[test]
    async fn forwarder() {
        let sender = UdpSocket::default().await.unwrap();
        let sender_address = sender.as_ref().local_addr().unwrap();

        let forwarder = UdpSocket::default().await.unwrap();
        let forwarder_address = forwarder.as_ref().local_addr().unwrap();

        let receiver = UdpSocket::default().await.unwrap();
        let receiver_address = receiver.as_ref().local_addr().unwrap();

        let packets = ["hello", "goodbye"]
            .iter()
            .map(|data| AddressedUdp {
                address: forwarder_address,
                udp: data.as_bytes().into(),
            })
            .collect::<Vec<_>>();

        let sending = stream::iter(packets.clone()).map(Ok).forward(sender);
        let (abort_handle, abort_registration) = AbortHandle::new_pair();

        let (forwarder, mut config_writer) = Forwarder::new(forwarder);
        let forwarding = Abortable::new(forwarder.forward(), abort_registration);
        let receiving = async {
            let packets = receiver.take(2).collect::<Vec<_>>().await;
            abort_handle.abort();
            packets
        };

        config_writer
            .insert(sender_address.into(), receiver_address.into())
            .refresh();

        let (sending, forwarding, receiving) = join!(sending, forwarding, receiving);
        assert!(matches!(sending, Ok(_)));
        assert!(matches!(forwarding, Err(Aborted)));
        receiving
            .into_iter()
            .zip(packets)
            .for_each(|(received, expected)| {
                let received = received.unwrap();
                assert_eq!(received.udp, expected.udp);
            })
    }
}
