use std::net::{Ipv4Addr, Ipv6Addr};
use tokio_native_tls::TlsStream;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing as LOG;
use tokio_native_tls::TlsConnector;
use native_tls::TlsConnector as NativeTlsConnector;
use tokio::io::AsyncWriteExt;
use anyhow::Result;
use sha2::{Sha224, Digest};
use crate::settings::{ServerInfo};

/// Trojan Protocol:
///
/// ```text
/// +-----------------------+---------+----------------+---------+----------+
/// | hex(SHA224(password)) |  CRLF   | Trojan Request |  CRLF   | Payload  |
/// +-----------------------+---------+----------------+---------+----------+
/// |          56           | X'0D0A' |    Variable    | X'0D0A' | Variable |
/// +-----------------------+---------+----------------+---------+----------+
///
/// where Trojan Request is a SOCKS5-like request:
///
/// +-----+------+----------+----------+
/// | CMD | ATYP | DST.ADDR | DST.PORT |
/// +-----+------+----------+----------+
/// |  1  |  1   | Variable |    2     |
/// +-----+------+----------+----------+
///
/// where:
///   - CMD
///     - CONNECT X'01'
///     - UDP ASSOCIATE X'03'
///   - ATYP address type of following address
///     - IP V4 address: X'01'
///     - DOMAINNAME: X'03'
///     - IP V6 address: X'04'
///   - DST.ADDR desired destination address
///   - DST.PORT desired destination port in network octet order
/// ```

pub async fn is_tls_alive(tls: &mut TlsStream<TcpStream>) -> bool {
    match tls.write(&[]).await {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!("TLS connection dead: {}", e);
            false
        }
    }
}


#[derive(Clone)]
pub struct TrojanUtil {
}

impl TrojanUtil {

    // Safely split "host:port" (including IPv6 [::1]:443) into (host, port)
    pub fn split_host_port(addr: &str) -> Result<(String, u16)> {
        if let Some(host_end) = addr.rfind(']') {
            // Such as [2001:db8::1]:443
            let host_start = addr.find('[').ok_or_else(|| anyhow::anyhow!("invalid IPv6 bracket"))?;
            let host = &addr[host_start + 1 .. host_end];
            let port = addr[host_end+2..].parse::<u16>()?;
            return Ok((host.to_string(), port));
        }
        // Otherwise, use the last colon to separate the fields (to avoid IPv6 multi-colon issues).
        let (h, p) = addr.rsplit_once(':').ok_or_else(|| anyhow::anyhow!("missing port"))?;
        Ok((h.to_string(), p.parse::<u16>()?))
    }

    pub async fn create_connection(conf: &ServerInfo) -> Result<TlsStream<TcpStream>> {
        let trojan_addr = format!("{}:{}", conf.host, conf.port);
        let tcp = timeout(Duration::from_secs(5), TcpStream::connect(trojan_addr)).await??;
        let connector = NativeTlsConnector::builder().build()?;
        let connector = TlsConnector::from(connector);
        let tls = connector.connect(conf.sni.as_str(), tcp).await?;
        LOG::debug!("TLS handshake completed, peer cert: is {}", tls.get_ref().peer_certificate().is_ok());
        Ok(tls)
    }

    pub async fn send_trojan_request(password: &str, tls: &mut TlsStream<TcpStream>, target_addr:&str, port_u16: u16) -> Result<()> {
        let mut req = Vec::new();
        let mut hasher = Sha224::new();
        hasher.update(password.as_bytes());
        let hash = hasher.finalize();
        let password_hex = hex::encode(hash); // 56 bytes ASCII

        // 2) First send hex(SHA224(password)) + CRLF
        req.extend_from_slice(password_hex.as_bytes());
        req.extend_from_slice(b"\r\n");

        // 3) Building a Trojan Request (SOCKS5-like)
        //let (host, port_u16) = Self::split_host_port(format!("{}:{}", target_addr, tport).as_str())?;
        let host = target_addr;
        // CMD: CONNECT = 0x01
        req.push(0x01);

        // ATYP + ADDR
        if let Ok(ipv4) = host.parse::<Ipv4Addr>() {
            req.push(0x01);                   // IPv4
            req.extend_from_slice(&ipv4.octets());
        } else if let Ok(ipv6) = host.parse::<Ipv6Addr>() {
            req.push(0x04);                   // IPv6
            req.extend_from_slice(&ipv6.octets());
        } else {
            let host_bytes = host.as_bytes();
            if host_bytes.len() > 255 {
                anyhow::bail!("domain too long for trojan");
            }
            req.push(0x03);                   // DOMAIN
            req.push(host_bytes.len() as u8);
            req.extend_from_slice(host_bytes);
        }

        // PORT (network order)
        req.extend_from_slice(&port_u16.to_be_bytes());

        // Add a CRLF at the end according to the specification
        req.extend_from_slice(b"\r\n");

        // 4) Send Trojan Request
        tls.write_all(&req).await?;
        tls.flush().await?;
        // LOG::info!("Trojan proxy: host={}, port={}", host, port_u16);
        Ok(())
    }
}
