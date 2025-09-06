use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::Result;
use tokio::io;
use tracing as LOG;

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

    //io::copy_bidirectional(&mut inbound, &mut outbound).await?;

    let (mut ri, mut wi) = inbound.into_split();
    let (mut ro, mut wo) = outbound.into_split();


    let client_to_server = tokio::spawn(async move {
        io::copy(&mut ri, &mut wo).await?;
        wo.shutdown().await
    });

    let server_to_client = tokio::spawn(async move {
        io::copy(&mut ro, &mut wi).await?;
        wi.shutdown().await
    });

    let _ = tokio::try_join!(client_to_server, server_to_client)?;
    Ok(())
}


///  This is a front-end transparent proxy that forwards all requests to 127.0.0.1:8888 to the destination 127.0.0.1:20172.
///
///  Essentially, this is a simple port forwarding proxy.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:8888").await?;
    LOG::info!("Listening proxying on 127.0.0.1:8080");
    loop {
        let (inbound, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = transfer(inbound, "127.0.0.1:20172").await {
                tracing::error!("Transfer error: {:?}", e);
            }
        });
    }
}
