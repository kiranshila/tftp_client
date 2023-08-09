//! Parser and serialization of the TFTP [`Packet`]

use byte_strings::c_str;
use std::{
    ffi::{CStr, CString},
    fmt::Display,
};
use thiserror::Error;

#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    Unspec = 0,
    NoFile = 1,
    Access = 2,
    Write = 3,
    Op = 4,
    BadId = 5,
    Exist = 6,
    BadUser = 7,
    BadOpt = 8,
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::Unspec => write!(f, "Not defined, see error message"),
            ErrorCode::NoFile => write!(f, "File not found"),
            ErrorCode::Access => write!(f, "Access violation"),
            ErrorCode::Write => write!(f, "Disk full or allocation exceeded"),
            ErrorCode::Op => write!(f, "Illegal TFTP operation"),
            ErrorCode::BadId => write!(f, "Unknown transfer ID"),
            ErrorCode::Exist => write!(f, "File already exists"),
            ErrorCode::BadUser => write!(f, "No such user"),
            ErrorCode::BadOpt => write!(f, "Bad option"),
        }
    }
}

impl ErrorCode {
    fn from_u16(v: u16) -> Result<Self, Error> {
        Ok(match v {
            0 => ErrorCode::Unspec,
            1 => ErrorCode::NoFile,
            2 => ErrorCode::Access,
            3 => ErrorCode::Write,
            4 => ErrorCode::Op,
            5 => ErrorCode::BadId,
            6 => ErrorCode::Exist,
            7 => ErrorCode::BadUser,
            8 => ErrorCode::BadOpt,
            _ => return Err(Error::BadErrorCode(v)),
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RequestMode {
    Octet,
    NetAscii,
    Mail,
}

impl Display for RequestMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestMode::Octet => write!(f, "octet"),
            RequestMode::NetAscii => write!(f, "netascii"),
            RequestMode::Mail => write!(f, "mail"),
        }
    }
}

impl RequestMode {
    fn from_cstr(str: &CStr) -> Result<Self, Error> {
        Ok(
            match str
                .to_str()
                .map_err(|_| Error::BadString)?
                .to_ascii_lowercase()
                .as_str()
            {
                "octet" => Self::Octet,
                "netascii" => Self::NetAscii,
                "mail" => Self::Mail,
                _ => return Err(Error::BadString),
            },
        )
    }

    fn into_cstr(self) -> &'static CStr {
        match self {
            RequestMode::Octet => c_str!("octet"),
            RequestMode::NetAscii => c_str!("netascii"),
            RequestMode::Mail => c_str!("mail"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    ReadRequest {
        filename: CString,
        mode: RequestMode,
    },
    WriteRequest {
        filename: CString,
        mode: RequestMode,
    },
    Data {
        block_n: u16,
        data: Vec<u8>,
    },
    Acknowledgment {
        block_n: u16,
    },
    Error {
        code: ErrorCode,
        msg: CString,
    },
}

impl Display for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Packet::ReadRequest { filename, mode } => {
                write!(f, "RRQ {} {mode}", filename.to_str().unwrap())
            }
            Packet::WriteRequest { filename, mode } => {
                write!(f, "WRQ {} {mode}", filename.to_str().unwrap())
            }
            Packet::Data { block_n, data: _ } => write!(f, "DATA block:{block_n}"),
            Packet::Acknowledgment { block_n } => write!(f, "ACK block:{block_n}"),
            Packet::Error { code, msg } => {
                write!(f, "ERROR code:{code} msg:{}", msg.to_str().unwrap())
            }
        }
    }
}

impl Packet {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![];
        match self {
            Packet::ReadRequest { filename, mode } => {
                buf.extend_from_slice(&1u16.to_be_bytes());
                buf.extend_from_slice(filename.to_bytes_with_nul());
                buf.extend_from_slice(mode.into_cstr().to_bytes_with_nul());
            }
            Packet::WriteRequest { filename, mode } => {
                buf.extend_from_slice(&2u16.to_be_bytes());
                buf.extend_from_slice(filename.to_bytes_with_nul());
                buf.extend_from_slice(mode.into_cstr().to_bytes_with_nul());
            }
            Packet::Data { block_n, data } => {
                buf.extend_from_slice(&3u16.to_be_bytes());
                buf.extend_from_slice(&block_n.to_be_bytes());
                buf.extend_from_slice(data);
            }
            Packet::Acknowledgment { block_n } => {
                buf.extend_from_slice(&4u16.to_be_bytes());
                buf.extend_from_slice(&block_n.to_be_bytes());
            }
            Packet::Error { code, msg } => {
                buf.extend_from_slice(&5u16.to_be_bytes());
                buf.extend_from_slice(&(*code as u16).to_be_bytes());
                buf.extend_from_slice(msg.as_bytes_with_nul());
            }
        }
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 4 {
            // Check against the smallest payload size (ACK)
            return Err(Error::Incomplete(bytes.len()));
        }
        // Now we're guaranteed to at least have the opcode
        let opcode = u16::from_be_bytes(bytes[0..2].try_into().unwrap());
        let body = &bytes[2..];
        match opcode {
            // RRQ
            1 => {
                // Smallest size after the opcode is 7 bytes
                // 2 bytes for 1 char filename and 5 bytes for "mail" mode
                if body.len() < 7 {
                    Err(Error::Incomplete(body.len()))
                } else {
                    // The rest should have exactly two null bytes, one for each string
                    let mut iter = body.splitn(3, |x| *x == 0);
                    let filename = iter.next().ok_or(Error::Incomplete(0))?;
                    let mode = iter.next().ok_or(Error::Incomplete(0))?;
                    Ok(Packet::ReadRequest {
                        filename: CString::new(filename).map_err(|_| Error::BadString)?,
                        mode: RequestMode::from_cstr(
                            &CString::new(mode).map_err(|_| Error::BadString)?,
                        )?,
                    })
                }
            }
            // WRQ
            2 => {
                // Same story as RRQ, but different discriminant
                if body.len() < 7 {
                    Err(Error::Incomplete(body.len()))
                } else {
                    // The rest should have exactly two null bytes, one for each string
                    let mut iter = body.splitn(3, |x| *x == 0);
                    let filename = iter.next().ok_or(Error::Incomplete(0))?;
                    let mode = iter.next().ok_or(Error::Incomplete(0))?;
                    Ok(Packet::WriteRequest {
                        filename: CString::new(filename).map_err(|_| Error::BadString)?,
                        mode: RequestMode::from_cstr(
                            &CString::new(mode).map_err(|_| Error::BadString)?,
                        )?,
                    })
                }
            }
            // DATA
            3 => {
                // Minimum data body size is a block num of 2 bytes and 0 data bytes,
                if body.len() < 2 {
                    Err(Error::Incomplete(body.len()))
                } else {
                    let block_n = u16::from_be_bytes(body[..2].try_into().unwrap());
                    let data = body[2..].to_vec();
                    Ok(Packet::Data { block_n, data })
                }
            }
            // ACK
            4 => {
                // We've already checked length for this smallest payload
                let block_n = u16::from_be_bytes(body[..2].try_into().unwrap());
                Ok(Packet::Acknowledgment { block_n })
            }
            // ERROR
            5 => {
                // Minimum size here is 3 bytes, 2 for the error code and 1 for a zero length string (null byte)
                if body.len() < 3 {
                    Err(Error::Incomplete(body.len()))
                } else {
                    let code =
                        ErrorCode::from_u16(u16::from_be_bytes(body[0..2].try_into().unwrap()))?;
                    // The rest should have exactly one null byte at the end for the string
                    if *body[2..].last().unwrap() != 0 {
                        Err(Error::BadString)
                    } else {
                        let msg = CString::new(&body[2..(body.len() - 1)])
                            .map_err(|_| Error::BadString)?;
                        Ok(Packet::Error { code, msg })
                    }
                }
            }
            _ => Err(Error::BadOpcode(opcode)),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Too few bytes recieved - `{0}`")]
    Incomplete(usize),
    #[error("Opcode wasn't expected - `{0}`")]
    BadOpcode(u16),
    #[error("String in payload wasn't a valid CString or was otherwise invalid")]
    BadString,
    #[error("Error code wasn't recognized - `{0}`")]
    BadErrorCode(u16),
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use paste::paste;

    macro_rules! test_happy_packet {
        ($packet:expr, $name:literal) => {
            paste! {
                #[test]
                fn [<test_ $name>]() {
                    let pkt = $packet;
                    // Serialize to bytes
                    let bytes = pkt.to_bytes();
                    // And back to packet
                    let pkt_parsed = Packet::from_bytes(&bytes).unwrap();
                    // And check
                    assert_eq!(pkt, pkt_parsed);
                }
            }
        };
    }

    test_happy_packet! {Packet::ReadRequest {filename:CString::new("foo").unwrap(), mode: RequestMode::Octet}, "rrq_octet"}
    test_happy_packet! {Packet::ReadRequest {filename:CString::new("foo").unwrap(), mode: RequestMode::NetAscii}, "rrq_netascii"}
    test_happy_packet! {Packet::ReadRequest {filename:CString::new("foo").unwrap(), mode: RequestMode:: Mail}, "rrq_mail"}
    test_happy_packet! {Packet::WriteRequest {filename:CString::new("foo").unwrap(), mode: RequestMode::Octet}, "wrq_octet"}
    test_happy_packet! {Packet::WriteRequest {filename:CString::new("foo").unwrap(), mode: RequestMode::NetAscii}, "wrq_netascii"}
    test_happy_packet! {Packet::WriteRequest {filename:CString::new("foo").unwrap(), mode: RequestMode:: Mail}, "wrq_mail"}
    test_happy_packet! {Packet::Data {block_n: 42, data: vec![0xDE, 0xAD, 0xBE, 0xEF]}, "data"}
    test_happy_packet! {Packet::Data {block_n: 123, data: vec![]}, "data_empty"}
    test_happy_packet! {Packet::Acknowledgment { block_n: 42 }, "ack"}
    test_happy_packet! {Packet::Error { code: ErrorCode::Unspec, msg: CString::new("Msg").unwrap() }, "error_unspec"}
    test_happy_packet! {Packet::Error { code: ErrorCode::NoFile, msg: CString::new("Msg").unwrap() }, "error_nofile"}
    test_happy_packet! {Packet::Error { code: ErrorCode::Access, msg: CString::new("Msg").unwrap() }, "error_access"}
    test_happy_packet! {Packet::Error { code: ErrorCode::Write, msg: CString::new("Msg").unwrap() }, "error_write"}
    test_happy_packet! {Packet::Error { code: ErrorCode::Op, msg: CString::new("Msg").unwrap() }, "error_op"}
    test_happy_packet! {Packet::Error { code: ErrorCode::BadId, msg: CString::new("Msg").unwrap() }, "error_badid"}
    test_happy_packet! {Packet::Error { code: ErrorCode::Exist, msg: CString::new("Msg").unwrap() }, "error_exist"}
    test_happy_packet! {Packet::Error { code: ErrorCode::BadUser, msg: CString::new("Msg").unwrap() }, "error_baduser"}
    test_happy_packet! {Packet::Error { code: ErrorCode::BadOpt, msg: CString::new("Msg").unwrap() }, "error_badopt"}
    test_happy_packet! {Packet::Error { code: ErrorCode::BadOpt, msg: CString::new("").unwrap() }, "error_empty"}
}
