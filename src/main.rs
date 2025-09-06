use anyhow::{Context, Result};
use native_tls::TlsConnector as NativeTlsConnector;
use proxy_rs::json_reader::Config;
use proxy_rs::trojan_util::TrojanUtil;
use proxy_rs::{json_reader, socks5_helper};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::{Arc, OnceLock};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::OnceCell;
use tokio_native_tls::{TlsConnector, TlsStream};
use tracing as LOG;

async fn handle_client(client_stream: TcpStream) -> Result<()> {
    let (inbound, target, tport) = socks5_helper::handle_socks5(client_stream).await?;
    transfer_to_trojan(inbound, &target, tport).await
}

static CONFIG: OnceCell<Arc<Config>> = OnceCell::const_new();
async fn get_config() -> Arc<Config> {
    CONFIG
        .get_or_init(|| async {
            let config: Config =
                json_reader::load_json("proxy.cache.json").expect("failed to load config");
            let index = config.select[0];
            let conf = config
                .list
                .get(index as usize - 1)
                .ok_or_else(|| anyhow::anyhow!("Index {} out of bounds", index))
                .expect("index out of bounds");
            if !conf.scheme.eq_ignore_ascii_case("trojan") {
                anyhow::anyhow!("server index check failed");
            }
            if conf.index != index {
                anyhow::anyhow!("server index check failed");
            }
            LOG::info!("** Usage trojan server {} **", conf.name);
            Arc::new(config)
        })
        .await
        .clone()
}

async fn transfer_to_trojan(mut inbound: TcpStream, target_addr: &str, tport: u16) -> Result<()> {
    let config = get_config().await;
    let index = config.select[0];
    let conf = config
        .list
        .get(index as usize - 1)
        .ok_or_else(|| anyhow::anyhow!("Index {} out of bounds", index))?;
    let mut tls = TrojanUtil::create_connection(conf).await?;

    // 1. Connect to the Trojan server
    let (host, port_u16) =
        TrojanUtil::send_trojan_request(conf.key.as_str(), &mut tls, target_addr, tport).await?;

    // 2. Bidirectional data forwarding
    let (from_client, from_server) = io::copy_bidirectional(&mut inbound, &mut tls)
        .await
        .context(format!("Transfer failed, host {}:{}", host, port_u16))?;

    LOG::info!(
        "Trojan proxy: host={}, port={}, ransferred {} bytes from client, {} bytes from server",
        host,
        port_u16,
        from_client,
        from_server
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Listening socks port
    let listener = TcpListener::bind("127.0.0.1:20172").await?;
    LOG::info!("Listening socks prot on 127.0.0.1:20172");
    loop {
        let (inbound, addr) = listener.accept().await?;
        // LOG::info!("New client from {}", addr);
        tokio::spawn(async move {
            if let Err(e) = handle_client(inbound).await {
                LOG::error!("socks Proxy error: {}", e);
            }
        });
    }
}
