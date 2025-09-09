use anyhow::{Context, Result};
use native_tls::TlsConnector as NativeTlsConnector;
use proxy_rs::settings::{Config, ServerInfo};
use proxy_rs::trojan_util::TrojanUtil;
use proxy_rs::{http_helper, settings, socks5_helper};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::{Arc, OnceLock};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{futures, OnceCell};
use tokio_native_tls::{TlsConnector, TlsStream};
use tracing as LOG;

async fn handle_http(client_stream: TcpStream) -> Result<()> {
    let (inbound, target, tport) = http_helper::handle_http_https(client_stream).await?;
    LOG::info!("received http CONNECT {}:{}", target, tport);

    transfer_to_trojan(inbound, &target, tport, "http").await
}

async fn handle_socks(client_stream: TcpStream) -> Result<()> {
    let (inbound, target, tport) = socks5_helper::handle_socks5(client_stream).await?;
    LOG::info!("received SOCKS5 CONNECT {}:{}", target, tport);

    transfer_to_trojan(inbound, &target, tport, "socks").await
}

async fn transfer_to_trojan(
    mut inbound: TcpStream,
    target_addr: &str,
    tport: u16,
    mode: &str,
) -> Result<()> {
    let info: &ServerInfo = settings::get_trojan_server(target_addr).await?;
    let mut tls = TrojanUtil::create_connection(info).await?;

    // 1. Connect to the Trojan server
    let (host, port_u16) =
        TrojanUtil::send_trojan_request(info.key.as_str(), &mut tls, target_addr, tport).await?;

    // 2. Bidirectional data forwarding
    let (from_client, from_server) = io::copy_bidirectional(&mut inbound, &mut tls)
        .await
        .context(format!("Transfer failed, host {}:{}", host, port_u16))?;

    LOG::info!(
        "trojan accepted tcp:{}:{} [{mode} > proxy], transferred {} bytes from client, {} bytes from server",
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
    let socks_addr = settings::get_config().socks_bind.as_str();
    let http_addr = settings::get_config().http_bind.as_str();

    let socks_listener = TcpListener::bind(socks_addr).await?;
    LOG::info!("Listening socks on {}", socks_addr);
    let socks_handle = tokio::spawn(async move {
        loop {
            match socks_listener.accept().await {
                Ok((inbound, _)) => {
                    tokio::spawn(async move {
                        if let Err(e) = handle_socks(inbound).await {
                            LOG::error!("Error on {}: {}", socks_addr, e);
                        }
                    });
                }
                Err(e) => LOG::error!("Accept error on {}: {}", socks_addr, e),
            }
        }
    });
    //------------------------
    let http_listener = TcpListener::bind(http_addr).await?;
    LOG::info!("Listening http on {}", http_addr);
    let http_handle = tokio::spawn(async move {
        loop {
            match http_listener.accept().await {
                Ok((inbound, _)) => {
                    tokio::spawn(async move {
                        if let Err(e) = handle_http(inbound).await {
                            LOG::error!("Error on {}: {}", http_addr, e);
                        }
                    });
                }
                Err(e) => LOG::error!("Accept error on {}: {}", http_addr, e),
            }
        }
    });

    let _ = tokio::try_join!(socks_handle, http_handle)?;
    println!("Shutting down...");
    Ok(())
}
