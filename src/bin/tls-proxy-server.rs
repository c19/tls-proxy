extern crate tls_proxy;
use fast_socks5::server::{Config as SocksArgs};
use std::{fs};

use rustls_pemfile::{certs, rsa_private_keys};
use std::fs::File;
use std::io::{self, BufReader};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{copy, sink, split, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::rustls::{self, Certificate, PrivateKey};
use tokio_rustls::TlsAcceptor;
#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;
/// Tokio Rustls server example
#[derive(Serialize, Deserialize)]
pub struct TlsArgs {
    pub addr: String,
    pub cert: PathBuf,
    pub key: PathBuf,
    pub echo_mode: bool,
}

fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    rsa_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub tls: TlsArgs,
    pub socks5: SocksArgs,
}

async fn run_tls_server(args: TlsArgs){
    let addr = args
        .addr
        .to_socket_addrs().unwrap()
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable)).unwrap();
    let certs = load_certs(&args.cert).unwrap();
    let mut keys = load_keys(&args.key).unwrap();

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err)).unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(addr).await.unwrap();

    loop {
        let (stream, peer_addr) = listener.accept().await.unwrap();
        let acceptor = acceptor.clone();

        let fut = async move {
            let mut stream = acceptor.accept(stream).await?;
            // pass tls stream to socks stream

            let mut output = sink();
            stream
                .write_all(
                    &b"HTTP/1.0 200 ok\r\n\
                Connection: close\r\n\
                Content-length: 12\r\n\
                \r\n\
                Hello world!"[..],
                )
                .await?;
            stream.shutdown().await?;
            copy(&mut stream, &mut output).await?;
            println!("Hello: {}", peer_addr);

            Ok(()) as io::Result<()>
        };

        tokio::spawn(async move {
            if let Err(err) = fut.await {
                eprintln!("{:?}", err);
            }
        });
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let yaml_raw = fs::read_to_string("src/bin/tls-proxy-server.yaml").expect("Unable to read file: src/bin/tls-proxy-server.yaml");
    let args: Config = serde_yaml::from_str(&yaml_raw).expect("unable to deserialize config yaml");

    run_tls_server(args.tls).await;
    Ok(())
}
