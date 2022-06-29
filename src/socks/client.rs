#[forbid(unsafe_code)]

use fast_socks5::{
    server::{Config, SimpleUserPassword, Socks5Socket},
    Result,
};
use log::{error, info};
use log::warn;
use std::future::Future;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::task;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
};

/// # How to use it:
///
/// Listen on a local address, authentication-free:
///     `$ RUST_LOG=debug cargo run --example simple_tcp_server -- --listen-addr 127.0.0.1:1337 no-auth`
///
/// Listen on a local address, with basic username/password requirement:
///     `$ RUST_LOG=debug cargo run --example simple_tcp_server -- --listen-addr 127.0.0.1:1337 password --username admin --password password`
///
#[derive(Debug, StructOpt)]
#[structopt(
    name = "socks5-server",
    about = "A simple implementation of a socks5-server."
)]
struct Opt {
    /// Bind on address address. eg. `127.0.0.1:1080`
    #[structopt(short, long)]
    pub listen_addr: String,

    /// Request timeout
    #[structopt(short = "t", long, default_value = "10")]
    pub request_timeout: u64,

    /// Choose authentication type
    #[structopt(subcommand, name = "auth")] // Note that we mark a field as a subcommand
    pub auth: AuthMode,
}

/// Choose the authentication type
#[derive(StructOpt, Debug)]
enum AuthMode {
    NoAuth,
    Password {
        #[structopt(short, long)]
        username: String,

        #[structopt(short, long)]
        password: String,
    },
}

pub async fn spawn_socks_server() -> Result<()> {
    let opt: Opt = Opt::from_args();
    let mut config = Config::default();
    config.set_request_timeout(opt.request_timeout);

    match opt.auth {
        AuthMode::NoAuth => warn!("No authentication has been set!"),
        AuthMode::Password { username, password } => {
            config.set_authentication(SimpleUserPassword { username, password });
            info!("Simple auth system has been set.");
        }
    }

    let s = serde_yaml::to_string(&config).unwrap();
    dbg!(s);

    let config = Arc::new(config);

    let listener = TcpListener::bind(&opt.listen_addr).await?;
    //    listener.set_config(config);

    info!("Listen for socks connections @ {}", &opt.listen_addr);

    // Standard TCP loop
    loop {
        match listener.accept().await {
            Ok((socket, _addr)) => {
                info!("Connection from {}", socket.peer_addr()?);
                let socket = Socks5Socket::new(socket, config.clone());

                spawn_and_log_error(socket.upgrade_to_socks5());
            }
            Err(err) => error!("accept error = {:?}", err),
        }
    }
}

fn spawn_and_log_error<F, T>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<Socks5Socket<T>>> + Send + 'static,
    T: AsyncRead + AsyncWrite + Unpin,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            error!("{:#}", &e);
        }
    })
}
