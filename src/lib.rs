//! An implementation of the TFTP Client as specified in [RFC 1350](https://datatracker.ietf.org/doc/html/rfc1350)
//! This includes retries and timeouts with exponential backoff

use thiserror::Error;

#[cfg(feature = "async")]
pub mod asynchronous;
mod blocking;
pub mod parser;

/// The blocking functions are the default
pub use blocking::*;

const BLKSIZE: usize = 512;

enum State {
    Send,
    SendAgain,
    Recv,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Bad filename (not a valid CString)")]
    BadFilename,
    #[error("Socket IO error - `{0}`")]
    SocketIo(std::io::Error),
    #[error("Timeout while trying to complete transaction")]
    Timeout,
    #[error("Failed to parse incoming packet - `{0}`")]
    Parse(parser::Error),
    #[error("The packet we got back was unexpected")]
    UnexpectedPacket(parser::Packet),
    #[error("The protocol itself gave us an error with code `{code:?}`and msg `{msg}`")]
    Protocol {
        code: parser::ErrorCode,
        msg: String,
    },
}
