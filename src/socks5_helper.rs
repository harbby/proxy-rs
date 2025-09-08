use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing as LOG;

pub async fn handle_socks5(mut inbound: TcpStream) -> anyhow::Result<(TcpStream, String, u16)> {
    let mut buf = [0u8; 2];
    // ========== Handshake Phase ==========
    let n = inbound.read(&mut buf).await?;
    if n < 2 || buf[0] != 0x05 {
        anyhow::bail!("Not a SOCKS5 request");
    }

    let nmethods = buf[1] as usize;
    let mut methods = vec![0u8; nmethods];
    inbound.read_exact(&mut methods).await?;
    if !methods.contains(&0x00) {
        anyhow::bail!("No supported auth method");
    }

    // Reply: Select "No Authentication"
    inbound.write_all(&[0x05, 0x00]).await?;

    // ========== Request phase ==========
    let mut buf = [0u8; 4];
    inbound.read_exact(&mut buf).await?;
    if buf[0] != 0x05 {
        anyhow::bail!("Invalid SOCKS5 request");
    }
    if buf[1] != 0x01 {
        LOG::error!("Unsupported command: {}", buf[1]);
        return Err(anyhow::anyhow!("Only CONNECT is supported"));
    }
    let atyp = buf[3];
    let mut addr = String::new();
    let mut port: u16 = 0;
    LOG::debug!("SOCKS5 received SOCKS5 request");

    match atyp {
        0x01 => {
            // IPv4
            let mut buf = [0u8; 6];
            inbound.read_exact(&mut buf).await?;
            let ip = format!("{}.{}.{}.{}", buf[0], buf[1], buf[2], buf[3]);
            port = u16::from_be_bytes([buf[4], buf[5]]);
            addr = ip;
        }
        0x03 => {
            // Domain
            let mut len = [0u8; 1];
            inbound.read_exact(&mut len).await?;
            let mut domain = vec![0u8; len[0] as usize];
            inbound.read_exact(&mut domain).await?;
            let mut port_buf = [0u8; 2];
            inbound.read_exact(&mut port_buf).await?;
            port = u16::from_be_bytes(port_buf);
            addr = format!("{}", String::from_utf8_lossy(&domain));
        }
        0x04 => {
            // IPv6
            let mut buf = [0u8; 16];
            inbound.read_exact(&mut buf).await?;
            let ip = std::net::Ipv6Addr::from(buf);
            let mut buf = [0u8; 2];
            inbound.read_exact(&mut buf).await?;
            port = u16::from_be_bytes(buf);
            addr = format!("[{}]", ip);
        }
        _ => anyhow::bail!("Unknown ATYP"),
    }

    // LOG::debug!("received SOCKS5 CONNECT {}:{}", addr, port);
    // Successful response
    let resp = [
        0x05u8, 0x00, 0x00, 0x01, // VER, REP, RSV, ATYP=IPv4
        0, 0, 0, 0, // BND.ADDR (Here just return 0.0.0.0)
        0, 0, // BND.PORT
    ];
    inbound.write_all(&resp).await?;
    Ok((inbound, addr, port))
}
