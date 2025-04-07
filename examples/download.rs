use async_net::UdpSocket;
use futures_lite::future::block_on;
use std::time::Duration;

use tftp_client::download;

fn main() {
    let server = "192.168.0.3:69".parse().unwrap();

    let timeout = Duration::from_millis(100);
    let max_timeout = Duration::from_secs(5);
    let retries = 8;

    let bytes = block_on(async {
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        download("/temp", &socket, server, timeout, max_timeout, retries)
            .await
            .unwrap()
    });

    dbg!(f32::from_be_bytes(bytes.try_into().unwrap()));
}
