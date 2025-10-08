use anyhow::{Context, Result};
use proxy_rs::settings::ServerInfo;
use proxy_rs::trojan_util::TrojanUtil;
use proxy_rs::{http_helper, settings, socks5_helper};
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tracing as LOG;
use proxy_rs::router::is_no_proxy;

async fn handle_http(inbound: TcpStream) -> Result<()> {
    let mut buf = [0u8; 1];
    let n = inbound.peek(&mut buf).await?;
    if n == 0 {
        // Empty request
        return Ok(())
    }
    // 1 判断首字节是否 TLS Handshake
    if buf[0] == 22 {
        anyhow::bail!("Not supported by HTTPS, TLS handshake detected, rejecting connection");
    }
    let (inbound, target, port) = http_helper::handle_http(inbound).await?;
    // LOG::info!("received http CONNECT {}:{}", target, tport);
    transfer(inbound, &target, port, "http").await
}

async fn handle_socks(mut client_stream: TcpStream) -> Result<()> {
    // ========== Handshake Phase ==========
    let mut buf = [0u8; 2];
    let n = client_stream.read(&mut buf).await?;
    if n < 2 || buf[0] != 0x05 {
        // Empty request
        return Ok(())
    }

    let (inbound, target, port) = socks5_helper::handle_socks5(client_stream, buf[1]).await?;
    // LOG::info!("received SOCKS5 CONNECT {}:{}", target, tport);
    transfer(inbound, &target, port, "socks").await
}

async fn transfer(
    mut inbound: TcpStream,
    target_addr: &str,
    port: u16,
    mode: &str,
) -> Result<()> {
    let mut proxy_mode = "proxy";
    let (from_client, from_server) = if is_no_proxy(target_addr) {
        proxy_mode = "direct";
        let mut outbound = TcpStream::connect(format!("{target_addr}:{port}")).await
            .context(format!("Failed accepted tcp:{target_addr}:{port} [{mode} > {proxy_mode}]"))?;
        io::copy_bidirectional(&mut inbound, &mut outbound).await?
    } else {
        let info: &ServerInfo = settings::get_trojan_server(target_addr)?;
        let mut tls = TrojanUtil::create_connection(info).await
            .context(format!("Failed accepted trojan server [{}]{}", info.index, info.name))?;

        // 1. Connect to the Trojan server
        TrojanUtil::send_trojan_request(info.key.as_str(), &mut tls, target_addr, port).await?;

        // 2. Bidirectional data forwarding
        io::copy_bidirectional(&mut inbound, &mut tls)
            .await
            .context(format!(
                "trojan transfer failed, tcp:{}:{} [{mode} > {proxy_mode}]",
                target_addr, port
            ))?
    };

    LOG::info!(
        "accepted tcp:{}:{} [{mode} > {proxy_mode}], uplink:{} downlink:{} bytes",
        target_addr,
        port,
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
                            LOG::error!("{}", e);
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
                            LOG::error!("{}", e);
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
