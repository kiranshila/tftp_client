# tftp-client
> A pure-rust TFTP client library

There are several TFTP crates in the rust ecosystem:
- [tftp](https://crates.io/crates/tftp)
- [async-tftp](https://crates.io/crates/async-tftp)
- [tftp_server](https://crates.io/crates/tftp_server)
- [libtftp](https://crates.io/crates/libtftp)
- [tftp-ro](https://crates.io/crates/tftp-ro)
- [rtftp](https://crates.io/crates/rtftp)

All but the last only implement the server side. The last library seems focused on reimplementing the tftp applications, not so much focused on the rust library. Additionally, it is not as robust as the Python [tftpy](https://pypi.org/project/tftpy/) library.

This library, `tftp-client` implements only the client as per RFC 1350, including the fix for the ["sorcerer's spprentice syndrome"](https://en.wikipedia.org/wiki/Sorcerer%27s_Apprentice_Syndrome). It is blocking-only, doesn't implement any of the additional options, but provides robust control over how timeouts are handled. Unlike `rtftp`, retries include exponential backoff (with an upper limit) and have inner and outer retries for block-level and transfer level attempts.

