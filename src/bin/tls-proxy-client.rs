use mio::net::TcpStream;
use tls_proxy::tls::client::{USAGE, Args, lookup_ipv4, make_config, TlsClient};
use std::convert::TryInto;
use std::io;
use std::io::{Write};

#[macro_use]
extern crate serde_derive;

use docopt::Docopt;

/// Parse some arguments, then make a TLS client connection
/// somewhere.
fn main() {
    let version = env!("CARGO_PKG_NAME").to_string() + ", version: " + env!("CARGO_PKG_VERSION");

    let args: Args = Docopt::new(USAGE)
        .map(|d| d.help(true))
        .map(|d| d.version(Some(version)))
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_verbose {
        env_logger::Builder::new()
            .parse_filters("trace")
            .init();
    }

    let port = args.flag_port.unwrap_or(443);
    let addr = lookup_ipv4(args.arg_hostname.as_str(), port);

    let config = make_config(&args);

    let sock = TcpStream::connect(addr).unwrap();
    let server_name = args
        .arg_hostname
        .as_str()
        .try_into()
        .expect("invalid DNS name");
    let mut tlsclient = TlsClient::new(sock, server_name, config);

    if args.flag_http {
        let httpreq = format!(
            "GET / HTTP/1.0\r\nHost: {}\r\nConnection: \
                               close\r\nAccept-Encoding: identity\r\n\r\n",
            args.arg_hostname
        );
        tlsclient
            .write_all(httpreq.as_bytes())
            .unwrap();
    } else {
        let mut stdin = io::stdin();
        tlsclient
            .read_source_to_end(&mut stdin)
            .unwrap();
    }

    let mut poll = mio::Poll::new().unwrap();
    let mut events = mio::Events::with_capacity(32);
    tlsclient.register(poll.registry());

    loop {
        poll.poll(&mut events, None).unwrap();

        for ev in events.iter() {
            tlsclient.ready(ev);
            tlsclient.reregister(poll.registry());
        }
    }
}
