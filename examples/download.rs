use std::{net::UdpSocket, time::Duration};

use tftp_client::download;

fn main() -> std::io::Result<()> {
    let mut socket = UdpSocket::bind("192.168.0.3:69")?;

    let timeout = Duration::from_millis(100);
    let max_timeout = Duration::from_secs(5);
    let retries = 5;

    let bytes = download(
        "/dev/spec_a_vacc",
        &mut socket,
        timeout,
        max_timeout,
        retries,
    )
    .unwrap();

    dbg!(bytes);
    Ok(())
}
