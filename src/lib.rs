//! An implementation of the TFTP Client as specified in [RFC 1350](https://datatracker.ietf.org/doc/html/rfc1350)
//! This includes retries and timeouts with exponential backoff

use parser::Packet;
use std::{ffi::CString, io, net::UdpSocket, time::Duration};
use thiserror::Error;
use tracing::debug;

pub mod parser;

const BLKSISZE: usize = 512;

enum State {
    Send,
    SendAgain,
    Recv,
}

pub fn download<T: AsRef<str> + std::fmt::Display>(
    filename: T,
    socket: &mut UdpSocket,
    timeout: Duration,
    max_timeout: Duration,
    retries: usize,
) -> Result<Vec<u8>, Error> {
    // Make sure we can actually timeout, but preserve the old state
    let old_read_timeout = socket.read_timeout().map_err(Error::SocketIo)?;
    socket
        .set_read_timeout(Some(timeout))
        .map_err(Error::SocketIo)?;
    debug!("Starting download of {filename}");
    // Initialize the state of our state machine
    let mut state = State::Send;
    let mut local_retries = retries;
    let mut local_timeout = timeout;
    let mut send_pkt = Packet::ReadRequest {
        filename: CString::new(filename.to_string()).map_err(|_| Error::BadFilename)?,
        mode: parser::RequestMode::Octet,
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
                debug!("Sending bytes - {:?}", bytes);
                // Send the bytes and reset some other state variables
                let _n = socket.send(&bytes).map_err(Error::SocketIo)?;
                // Transition to recv if this wasn't the last ACK packet
                if done {
                    break;
                } else {
                    state = State::Recv
                }
            }
            State::SendAgain => {
                let bytes = send_pkt.to_bytes();
                debug!("Retry sending bytes - {:?}", bytes);
                // Send the bytes and reset some other state variables
                socket.send(&bytes).map_err(Error::SocketIo)?;
                // Transition to recv
                state = State::Recv
            }
            State::Recv => {
                let mut buf = vec![0; BLKSISZE + 4]; // The biggest a block can be, 2 bytes for opcode, 2 bytes for block n
                let n = match socket.recv(&mut buf) {
                    Ok(n) => n,
                    Err(e) => {
                        match e.kind() {
                            io::ErrorKind::TimedOut => {
                                // We timed out, try sending the last packet again with exponential backoff
                                local_retries -= 1;
                                if local_retries == 0 {
                                    return Err(Error::Timeout);
                                }
                                local_timeout += local_timeout / 2;
                                if local_timeout > max_timeout {
                                    local_timeout = max_timeout;
                                }
                                socket
                                    .set_read_timeout(Some(local_timeout))
                                    .map_err(Error::SocketIo)?;
                                state = State::SendAgain;
                                continue;
                            }
                            _ => return Err(Error::SocketIo(e)),
                        }
                    }
                };
                // Process the received packet
                let recv_pkt = Packet::from_bytes(&buf[..n]).map_err(Error::Parse)?;
                match recv_pkt {
                    Packet::Data { block_n, data } => {
                        // We got back a chunk of data, we need to ack it and append to the data we're collecting
                        file_data.extend_from_slice(&data);
                        if data.len() < BLKSISZE {
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
    // Return socket timeout to previous state
    socket
        .set_read_timeout(old_read_timeout)
        .map_err(Error::SocketIo)?;
    // And return the bytes we downloaded
    Ok(file_data)
}

pub fn upload<T: AsRef<str> + std::fmt::Display>(
    filename: T,
    data: &[u8],
    socket: &mut UdpSocket,
    timeout: Duration,
    max_timeout: Duration,
    retries: usize,
) -> Result<(), Error> {
    // Make sure we can actually timeout, but preserve the old state
    let old_read_timeout = socket.read_timeout().map_err(Error::SocketIo)?;
    socket
        .set_read_timeout(Some(timeout))
        .map_err(Error::SocketIo)?;
    debug!("Starting upload of data to {filename}");
    // Initialize the state of our state machine
    let mut state = State::Send;
    let mut local_retries = retries;
    let mut local_timeout = timeout;
    let mut send_pkt = Packet::WriteRequest {
        filename: CString::new(filename.to_string()).map_err(|_| Error::BadFilename)?,
        mode: parser::RequestMode::Octet,
    };
    // Create the chunk vec for our data
    let chunks: Vec<_> = data.chunks(BLKSISZE).collect();
    let mut last_block_n = -1;
    // Run the state machine
    loop {
        match state {
            State::Send => {
                local_retries = retries;
                local_timeout = timeout;
                let bytes = send_pkt.to_bytes();
                debug!("Sending bytes - {:?}", bytes);
                // Send the bytes and reset some other state variables
                let _n = socket.send(&bytes).map_err(Error::SocketIo)?;
                // Transition to recv if this wasn't the last ACK packet
                state = State::Recv;
            }
            State::SendAgain => {
                let bytes = send_pkt.to_bytes();
                debug!("Retry sending bytes - {:?}", bytes);
                // Send the bytes and reset some other state variables
                socket.send(&bytes).map_err(Error::SocketIo)?;
                // Transition to recv
                state = State::Recv
            }
            State::Recv => {
                let mut buf = vec![0; 4]; // The biggest a block can be, 2 bytes for opcode, 2 bytes for block n
                let n = match socket.recv(&mut buf) {
                    Ok(n) => n,
                    Err(e) => {
                        match e.kind() {
                            io::ErrorKind::TimedOut => {
                                // We timed out, try sending the last packet again with exponential backoff
                                local_retries -= 1;
                                if local_retries == 0 {
                                    return Err(Error::Timeout);
                                }
                                local_timeout += local_timeout / 2;
                                if local_timeout > max_timeout {
                                    local_timeout = max_timeout;
                                }
                                socket
                                    .set_read_timeout(Some(local_timeout))
                                    .map_err(Error::SocketIo)?;
                                state = State::Recv;
                                continue;
                            }
                            _ => return Err(Error::SocketIo(e)),
                        }
                    }
                };
                // Process the received packet
                let recv_pkt = Packet::from_bytes(&buf[..n]).map_err(Error::Parse)?;
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
                        } else {
                            send_pkt = Packet::Data {
                                block_n: block_n + 1,
                                data: chunks[block_n as usize].into(),
                            };
                            state = State::Send;
                            continue;
                        }
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
    // Return socket timeout to previous state
    socket
        .set_read_timeout(old_read_timeout)
        .map_err(Error::SocketIo)?;
    // And return and ok
    Ok(())
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Bad filename (not a valid CString)")]
    BadFilename,
    #[error("Socket IO error - `{0}`")]
    SocketIo(std::io::Error),
    #[error("Timeout while trying to complete transaction")]
    Timeout,
    #[error("Failed to parse incoming packet")]
    Parse(parser::Error),
    #[error("The packet we got back was unexpected")]
    UnexpectedPacket(Packet),
    #[error(
        "The protocol itself gave us an error with code `{:?}`and msg `{msg}`",
        code
    )]
    Protocol {
        code: parser::ErrorCode,
        msg: String,
    },
}
