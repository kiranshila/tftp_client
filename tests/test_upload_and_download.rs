use std::{net::UdpSocket, time::Duration};
use tftp_client::{download, upload};

#[test]
fn download_upload() {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let server = "127.0.0.1:69".parse().unwrap();
    let timeout = Duration::from_millis(100);
    let max_timeout = Duration::from_secs(5);
    let retries = 8;
    let test_payload = vec![0xb0, 0xba, 0xca, 0xfe];
    upload(
        "/test",
        &test_payload,
        &socket,
        server,
        timeout,
        max_timeout,
        retries,
    )
    .unwrap();
    let res = download("/test", &socket, server, timeout, max_timeout, retries).unwrap();
    assert_eq!(test_payload, res);
}
