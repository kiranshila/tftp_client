use std::{net::UdpSocket, time::Duration};

use tftp_client::download;

fn main() {
    let mut socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("192.168.0.3:69").unwrap();

    let timeout = Duration::from_millis(100);
    let max_timeout = Duration::from_secs(5);
    let retries = 8;

    let bytes = download("/temp", &mut socket, timeout, max_timeout, retries).unwrap();

    dbg!(f32::from_be_bytes(bytes.try_into().unwrap()));
}
