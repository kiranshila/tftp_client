use std::time::Duration;

#[test]
fn download_upload() {
    use std::net::UdpSocket;
    use tftp_client::{download, upload};

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

#[test]
#[cfg(feature = "async")]
fn download_upload_async() {
    use async_net::UdpSocket;
    use tftp_client::asynchronous::{download, upload};

    futures_lite::future::block_on(async move {
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        let server = "127.0.0.1:69".parse().unwrap();
        let timeout = Duration::from_millis(100);
        let max_timeout = Duration::from_secs(5);
        let retries = 8;
        let test_payload = vec![0xde, 0xad, 0xca, 0xfe];
        upload(
            "/test-async",
            &test_payload,
            &socket,
            server,
            timeout,
            max_timeout,
            retries,
        )
        .await
        .unwrap();
        let res = download("/test-async", &socket, server, timeout, max_timeout, retries)
            .await
            .unwrap();
        assert_eq!(test_payload, res);
    });
}
