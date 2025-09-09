use crate::trojan_util::TrojanUtil;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub async fn handle_http(mut inbound: TcpStream) -> anyhow::Result<(TcpStream, String, u16)> {
    let mut buf_reader = BufReader::new(&mut inbound);

    // 读取请求行
    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line).await?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let uri = parts.next().unwrap_or("").to_string();
    let version = parts.next().unwrap_or("").to_string();
    if method != "CONNECT" {
        anyhow::bail!("Wrong HTTP method: {}", method);
    }

    // let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        let bytes_read = buf_reader.read_line(&mut line).await?;
        if bytes_read == 0 || line.trim().is_empty() {
            break;
        }
        // not parse header
        // if let Some((k, v)) = line.split_once(':') {
        //     headers.push((k.trim().to_string(), v.trim().to_string()));
        // }
    }

    inbound.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
    inbound.flush().await?;
    let (host, port) = TrojanUtil::split_host_port(uri.as_str())?;
    Ok((inbound, host, port))
}

pub async fn handle_http_https(mut inbound: TcpStream) -> anyhow::Result<(TcpStream, String, u16)> {
    // peek 前 1k 字节不消费流
    let mut buf = [0u8; 1];
    let n = inbound.peek(&mut buf).await?;
    if n == 0 {
        anyhow::bail!("Connection closed");
    }
    // 1️⃣ 判断首字节是否 TLS Handshake
    if buf[0] != 22 {
        return handle_http(inbound).await;
    }

    // TLS/HTTPS，直接拒绝
    anyhow::bail!("Not supported by HTTPS, TLS handshake detected, rejecting connection");
}