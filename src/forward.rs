#[cfg(test)]
mod tests {
    use async_std::test;
    use async_udp_stream::{AddressedUdp, UdpSocket};
    use futures::{
        future::{AbortHandle, Abortable, Aborted},
        join, stream, SinkExt, StreamExt,
    };

    #[test]
    async fn forwarder() {
        let sender = UdpSocket::default().await.unwrap();
        let _sender_address = sender.as_ref().local_addr().unwrap();

        let mut forwarder = UdpSocket::default().await.unwrap();
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
        let forwarding = Abortable::new(
            async {
                while let Some(Ok(mut packet)) = forwarder.next().await {
                    packet.address = receiver_address;
                    forwarder.send(packet).await.unwrap();
                }
            },
            abort_registration,
        );
        let receiving = async {
            let packets = receiver.take(2).collect::<Vec<_>>().await;
            abort_handle.abort();
            packets
        };
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
