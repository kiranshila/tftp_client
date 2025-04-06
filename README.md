# tftp_client
> A pure-rust TFTP client library

[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)](#license)
[![docs](https://img.shields.io/docsrs/tftp_client?logo=rust&style=flat-square)](https://docs.rs/tftp_client/latest/tftp_client/index.html)
[![rustc](https://img.shields.io/badge/rustc-1.78+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![build status](https://img.shields.io/github/actions/workflow/status/kiranshila/tftp_client/ci.yml?branch=main&style=flat-square&logo=github)](https://github.com/kiranshila/tftp_client/actions)
[![Codecov](https://img.shields.io/codecov/c/github/kiranshila/tftp_client?style=flat-square)](https://app.codecov.io/gh/kiranshila/tftp_client)

There are several TFTP crates in the rust ecosystem:
- [tftp](https://crates.io/crates/tftp)
- [async-tftp](https://crates.io/crates/async-tftp)
- [tftp_server](https://crates.io/crates/tftp_server)
- [libtftp](https://crates.io/crates/libtftp)
- [tftp-ro](https://crates.io/crates/tftp-ro)
- [rtftp](https://crates.io/crates/rtftp)

All but the last only implement the server side.
The last library seems focused on reimplementing the tftp applications,
not so much focused on the rust library.  
Additionally, it is not as robust as the Python [tftpy](https://pypi.org/project/tftpy/) library.

This library, `tftp-client` implements only the client as per RFC 1350,
including the fix for the ["sorcerer's apprentice syndrome"](https://en.wikipedia.org/wiki/Sorcerer%27s_Apprentice_Syndrome).  
Currently the library doesn't implement any of the additional options,
but provides robust control over how timeouts are handled.

Unlike `rtftp`, retries include exponential backoff (with an upper limit) and
have inner and outer retries for block-level and transfer level attempts.

