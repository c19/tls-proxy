
use mio::net::TcpStream;
use tls_proxy::tls::client::{Args as TlsArgs, lookup_ipv4, make_config, TlsClient};
use fast_socks5::server::{Config as SocksArgs};
use std::convert::TryInto;
use std::{io, fs};
use std::io::{Write};

#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub tls: TlsArgs,
    pub socks5: SocksArgs,
}

fn run_tls_client(args: TlsArgs){
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

// Opens a socks server, and send the socks packet through a tls proxy.
fn main() {
    let yaml_raw = fs::read_to_string("src/bin/tls-proxy-client.yaml").expect("Unable to read file: src/bin/tls-proxy-client.yaml");
    let args: Config = serde_yaml::from_str(&yaml_raw).expect("unable to deserialize config yaml");

    run_tls_client(args.tls);
}
