use anyhow::{Context, Result};
use rsfly::trojan_util::TrojanUtil;
use rsfly::{http_helper, router, settings, socks5_helper};
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use log as LOG;

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
    addr: &str,
    port: u16,
    mode: &str,
) -> Result<()> {
    let (label, option) = router::Router::get_factory().do_route(addr);
    match option {
        None => {
            let mut outbound = TcpStream::connect(format!("{addr}:{port}")).await
                .context(format!("Failed accepted tcp:{addr}:{port} [{mode} > direct]"))?;

            LOG::info!("accepted tcp:{addr}:{port} [{mode} > direct][{label}]");
            if let Ok((up, down)) = io::copy_bidirectional(&mut inbound, &mut outbound).await {
                LOG::debug!("succeed tcp:{addr}:{port} [{mode} > direct], up:{up} down:{down} bytes");
            }
        }
        Some(info) => {
            use rsfly::color::Colorize;
            let proxy_mode = format!("{mode} > proxy").yellow();
            let mut tls = TrojanUtil::create_connection(info).await
                .context(format!("connect failed, trojan server [{}]{}", info.index, info.name))?;

            // 1. Connect to the Trojan server
            TrojanUtil::send_trojan_request(info.key.as_str(), &mut tls, addr, port).await
                .context(format!("accepted failed, trojan server [{}]{}", info.index, info.name))?;

            LOG::info!("accepted tcp:{addr}:{port} [{proxy_mode}][{}]", info.index);
            // 2. Bidirectional data forwarding
            if let Ok((up, down)) = io::copy_bidirectional(&mut inbound, &mut tls).await {
                LOG::debug!("succeed tcp:{addr}:{port} [{proxy_mode}], up:{up} down:{down} bytes");
            }
        }
    }

    Ok(())
}

fn init_logger() {
    use chrono::Local;
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format(|buf, record| {
            let now = Local::now().format("%H:%M:%S%.3f%:z");
            let level_style = buf.default_level_style(record.level());
            let mut level_bytes = Vec::new();
            level_style.write_to(&mut level_bytes).unwrap();
            use std::io::Write;
            std::io::Write::write_all(&mut level_bytes, record.level().as_str().as_bytes()).unwrap();
            level_style.write_reset_to(&mut level_bytes).unwrap(); // close ansi
            let level_str = String::from_utf8(level_bytes).unwrap();

            writeln!(
                buf,
                "[{} {:<5} {}] {}",
                now,
                level_str,
                record.target(),
                record.args()
            )
        })
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();

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
