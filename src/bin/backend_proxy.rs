
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use anyhow::Result;
use tokio::io;
use tracing as LOG;
use proxy_rs::socks5_helper;

async fn transfer(mut inbound: TcpStream, addr: &str) -> Result<()> {
    let mut outbound = match TcpStream::connect(addr).await {
        Ok(s) => {
            LOG::info!("Connected to outbound {}", addr);
            s
        }
        Err(e) => {
            LOG::error!("Failed to connect to outbound {}: {}", addr, e);
            return Err(e.into());
        }
    };

    io::copy_bidirectional(&mut inbound, &mut outbound).await?;
    // let (mut ri, mut wi) = inbound.into_split();
    // let (mut ro, mut wo) = outbound.into_split();
    // let client_to_server = tokio::spawn(async move {
    //     io::copy(&mut ri, &mut wo).await?;
    //     wo.shutdown().await
    // });
    // let server_to_client = tokio::spawn(async move {
    //     io::copy(&mut ro, &mut wi).await?;
    //     wi.shutdown().await
    // });
    // let _ = tokio::try_join!(client_to_server, server_to_client)?;
    Ok(())
}

async fn handle_client(stream: TcpStream) -> Result<()> {
    let (inbound, target, prot) = socks5_helper::handle_socks5(stream).await?;
    transfer(inbound, format!("{}:{}", &target, prot).as_str()).await
}

/**
* This is a socks5 backend direct connection proxy software, which needs to be deployed on the server
*/
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:8888").await?;
    LOG::info!("Listening proxying on 127.0.0.1:8888");
    loop {
        let (inbound, addr) = listener.accept().await?;
        // LOG::info!("New client from {}", addr);

        tokio::spawn(async move {
            if let Err(e) = handle_client(inbound).await {
                tracing::error!("Transfer error: {:?}", e);
            }
        });
    }
}
