//! Asynchonous implementation of the TFTP client,
//! using [`smol-rs`](https://github.com/smol-rs/smol) components

use std::{
    ffi::CString,
    io::ErrorKind,
    net::SocketAddr,
    time::Duration,
};

use async_io::Timer;
use async_net::UdpSocket;
use futures_lite::FutureExt;
use tracing::debug;

use crate::{
    parser::{
        Packet,
        RequestMode,
    },
    Error,
    State,
    BLKSIZE,
};

/// Download a file via tftp
pub async fn download<T: AsRef<str> + std::fmt::Display>(
    filename: T,
    socket: &UdpSocket,
    mut server: SocketAddr,
    timeout: Duration,
    max_timeout: Duration,
    retries: usize,
) -> Result<Vec<u8>, Error> {
    // Set our server address to the inital address, it will potentially change
    debug!("┌── GET {filename}");
    // Initialize the state of our state machine
    let mut state = State::Send;
    let mut local_retries = retries;
    let mut local_timeout = timeout;
    let mut send_pkt = Packet::ReadRequest {
        filename: CString::new(filename.to_string()).map_err(|_| Error::BadFilename)?,
        mode: RequestMode::Octet,
    };
    let mut file_data = vec![];
    let mut done = false;
    // Run the state machine
    loop {
        match state {
            State::Send => {
                local_retries = retries;
                local_timeout = timeout;
                let bytes = send_pkt.to_bytes();
                debug!("│ TX - {send_pkt}");
                // Send the bytes and reset some other state variables
                socket
                    .send_to(&bytes, server)
                    .await
                    .map_err(Error::SocketIo)?;
                // Transition to recv if this wasn't the last ACK packet
                if done {
                    break;
                }
                state = State::Recv
            }
            State::SendAgain => {
                let bytes = send_pkt.to_bytes();
                debug!("│ TX - {send_pkt} (Retry)");
                // Send the bytes and reset some other state variables
                socket
                    .send_to(&bytes, server)
                    .await
                    .map_err(Error::SocketIo)?;
                // Transition to recv
                state = State::Recv
            }
            State::Recv => {
                let mut buf = vec![0; BLKSIZE + 4]; // The biggest a block can be, 2 bytes for opcode, 2 bytes for block n
                let n = match socket
                    .recv_from(&mut buf)
                    .or(async {
                        Timer::after(local_timeout).await;
                        Err(ErrorKind::TimedOut.into())
                    })
                    .await
                {
                    Ok((n, remote_addr)) => {
                        // Update the server's address as the spec allows the port to change
                        server = remote_addr;
                        n
                    }
                    Err(ref e) if e.kind() == ErrorKind::TimedOut => {
                        debug!("│ Timeout");
                        // Timeout, try sending the last packet again with exponential backoff
                        local_retries -= 1;
                        if local_retries == 0 {
                            return Err(Error::Timeout);
                        }
                        local_timeout += local_timeout / 2;
                        if local_timeout > max_timeout {
                            local_timeout = max_timeout;
                        }
                        state = State::SendAgain;
                        continue;
                    }
                    Err(e) => return Err(Error::SocketIo(e)),
                };
                // Process the received packet
                let recv_pkt = Packet::from_bytes(&buf[..n]).map_err(Error::Parse)?;
                debug!("│ RX - {recv_pkt}");
                match recv_pkt {
                    Packet::Data { block_n, data } => {
                        // We got back a chunk of data, we need to ack it and append to the data
                        // we're collecting
                        file_data.extend_from_slice(&data);
                        if data.len() < BLKSIZE {
                            done = true
                        }
                        send_pkt = Packet::Acknowledgment { block_n };
                        state = State::Send;
                        continue;
                    }
                    Packet::Error { code, msg } => {
                        return Err(Error::Protocol {
                            code,
                            msg: msg.into_string().expect("Error message had invalid UTF-8"),
                        })
                    }
                    _ => return Err(Error::UnexpectedPacket(recv_pkt)),
                }
            }
        }
    }
    debug!("└");
    Ok(file_data)
}

/// Upload a file via tftp
pub async fn upload<T: AsRef<str> + std::fmt::Display>(
    filename: T,
    data: &[u8],
    socket: &UdpSocket,
    mut server: SocketAddr,
    timeout: Duration,
    max_timeout: Duration,
    retries: usize,
) -> Result<(), Error> {
    debug!("┌── PUT {filename}");
    // Initialize the state of our state machine
    let mut state = State::Send;
    let mut local_retries = retries;
    let mut local_timeout = timeout;
    let mut send_pkt = Packet::WriteRequest {
        filename: CString::new(filename.to_string()).map_err(|_| Error::BadFilename)?,
        mode: RequestMode::Octet,
    };
    // Create the chunk vec for our data
    let chunks: Vec<_> = data.chunks(BLKSIZE).collect();
    let mut last_block_n = -1;
    // Run the state machine
    loop {
        match state {
            State::Send => {
                local_retries = retries;
                local_timeout = timeout;
                let bytes = send_pkt.to_bytes();
                debug!("│ TX - {send_pkt}");
                // Send the bytes and reset some other state variables
                socket
                    .send_to(&bytes, server)
                    .await
                    .map_err(Error::SocketIo)?;
                // Transition to recv if this wasn't the last ACK packet
                state = State::Recv;
            }
            State::SendAgain => {
                let bytes = send_pkt.to_bytes();
                debug!("│ TX - {send_pkt} (Retry)");
                // Send the bytes and reset some other state variables
                socket
                    .send_to(&bytes, server)
                    .await
                    .map_err(Error::SocketIo)?;
                // Transition to recv
                state = State::Recv
            }
            State::Recv => {
                let mut buf = vec![0; BLKSIZE + 4];

                let n = match socket
                    .recv_from(&mut buf)
                    .or(async {
                        Timer::after(local_timeout).await;
                        Err(ErrorKind::TimedOut.into())
                    })
                    .await
                {
                    Ok((n, remote_addr)) => {
                        // Update the server's address as the spec allows the port to change
                        server = remote_addr;
                        n
                    }
                    Err(ref e) if e.kind() == ErrorKind::TimedOut => {
                        debug!("│ Timeout");
                        // Timeout, try sending the last packet again with exponential backoff
                        local_retries -= 1;
                        if local_retries == 0 {
                            return Err(Error::Timeout);
                        }
                        local_timeout += local_timeout / 2;
                        if local_timeout > max_timeout {
                            local_timeout = max_timeout;
                        }
                        state = State::SendAgain;
                        continue;
                    }
                    Err(e) => return Err(Error::SocketIo(e)),
                };
                // Process the received packet
                let recv_pkt = Packet::from_bytes(&buf[..n]).map_err(Error::Parse)?;
                debug!("│ RX - {recv_pkt}");
                match recv_pkt {
                    Packet::Acknowledgment { block_n } => {
                        // Fix for https://en.wikipedia.org/wiki/Sorcerer%27s_Apprentice_Syndrome
                        // Just try to recv again and don't resend the data on duplicate Acks
                        if last_block_n == -1 {
                            // Initial block
                            last_block_n = block_n as i16
                        } else if last_block_n == block_n as i16 {
                            state = State::Recv;
                            continue;
                        } else {
                            last_block_n = block_n as i16;
                        }
                        // We got back an ack, we need to send out that ack's chunk of data
                        if block_n as usize == chunks.len() {
                            break;
                        }
                        send_pkt = Packet::Data {
                            block_n: block_n + 1,
                            data: chunks[block_n as usize].into(),
                        };
                        state = State::Send;
                        continue;
                    }
                    Packet::Error { code, msg } => {
                        return Err(Error::Protocol {
                            code,
                            msg: msg.into_string().expect("Error message had invalid UTF-8"),
                        })
                    }
                    _ => return Err(Error::UnexpectedPacket(recv_pkt)),
                }
            }
        }
    }
    debug!("└");
    Ok(())
}
