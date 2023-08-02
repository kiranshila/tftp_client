use std::{net::UdpSocket, time::Duration};

use tftp_client::upload;

fn main() {
    let mut socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("192.168.0.3:69").unwrap();

    let timeout = Duration::from_millis(100);
    let max_timeout = Duration::from_secs(5);
    let retries = 8;

    upload(
        "/dev/spec_a_vacc",
        &vec![0x69; 16384],
        &mut socket,
        timeout,
        max_timeout,
        retries,
    )
    .unwrap();
}
