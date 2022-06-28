use mio::net::{TcpListener};
extern crate tls_proxy;
use tls_proxy::tls::server::{LISTENER, ServerMode, TlsServer, Args, USAGE, make_config};

#[macro_use]
extern crate log;

use std::{net, fs};

#[macro_use]
extern crate serde_derive;

fn main() {
    let yaml_raw = fs::read_to_string("src/bin/tls-proxy-server.yaml").expect("Unable to read file: src/bin/tls-proxy-server.yaml");
    let args: Args = serde_yaml::from_str(&yaml_raw).expect("unable to deserialize config yaml");

    if args.flag_verbose {
        env_logger::Builder::new()
            .parse_filters("trace")
            .init();
    }

    let mut addr: net::SocketAddr = "0.0.0.0:443".parse().unwrap();
    addr.set_port(args.flag_port.unwrap_or(443));

    let config = make_config(&args);

    let mut listener = TcpListener::bind(addr).expect("cannot listen on port");
    let mut poll = mio::Poll::new().unwrap();
    poll.registry()
        .register(&mut listener, LISTENER, mio::Interest::READABLE)
        .unwrap();

    let mode = if args.cmd_echo {
        ServerMode::Echo
    } else if args.cmd_http {
        ServerMode::Http
    } else {
        ServerMode::Forward(args.arg_fport.expect("fport required"))
    };

    let mut tlsserv = TlsServer::new(listener, mode, config);

    let mut events = mio::Events::with_capacity(256);
    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            match event.token() {
                LISTENER => {
                    tlsserv
                        .accept(poll.registry())
                        .expect("error accepting socket");
                }
                _ => tlsserv.conn_event(poll.registry(), event),
            }
        }
    }
}
