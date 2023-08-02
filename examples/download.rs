use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use tftp_client::download;

fn main() {
    let mut socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("192.168.0.3:69").unwrap();

    let timeout = Duration::from_millis(100);
    let max_timeout = Duration::from_secs(5);
    let retries = 8;

    let bytes = download(
        "/dev/spec_a_vacc",
        &mut socket,
        timeout,
        max_timeout,
        retries,
    )
    .unwrap();

    dbg!(bytes);
}
