use async_net::UdpSocket;
use futures_lite::future;
use std::time::Duration;
use tftp_client::{
    download,
    upload,
};

#[test]
fn download_upload() {
    let server = "127.0.0.1:69".parse().unwrap();
    let timeout = Duration::from_millis(100);
    let max_timeout = Duration::from_secs(5);
    let retries = 8;
    let test_payload = vec![0xb0, 0xba, 0xca, 0xfe];
    let res = future::block_on(async {
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        upload(
            "/test",
            &test_payload,
            &socket,
            server,
            timeout,
            max_timeout,
            retries,
        )
        .await
        .unwrap();
        download("/test", &socket, server, timeout, max_timeout, retries)
            .await
            .unwrap()
    });
    assert_eq!(test_payload, res);
}
