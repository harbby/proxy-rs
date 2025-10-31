use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::io::Result;
use tokio::io;
use log as LOG;

async fn transfer(mut inbound: TcpStream, addr: &str) -> Result<()> {
    let local_addr = inbound.peer_addr()?;
    let local_port = local_addr.port();
    let local_host = local_addr.ip().to_string();

    let mut outbound = match TcpStream::connect(addr).await {
        Ok(s) => {
            LOG::info!("Connected to outbound {local_host}:{local_port} -> {}", addr);
            s
        }
        Err(e) => {
            LOG::error!("Failed to connect to outbound {}: {}", addr, e);
            return Err(e.into());
        }
    };
    // Method 1
    // let (mut ri, mut wi) = inbound.split();
    // let (mut ro, mut wo) = outbound.split();
    // let client_to_server = async  {
    //     io::copy(&mut ri, &mut wo).await?;
    //     wo.shutdown().await
    // };
    // let server_to_client = async {
    //     io::copy(&mut ro, &mut wi).await?;
    //     wi.shutdown().await
    // };
    // let _ = tokio::try_join!(client_to_server, server_to_client)?;
    // Method 2
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

    // Method 3
    let (from_client, from_server) = io::copy_bidirectional(&mut inbound, &mut outbound).await?;
    LOG::info!(
        "proxy accepted tcp:{}:{} -> {addr}, transferred {} bytes from client, {} bytes from server",
        local_host,
        local_port,
        from_client,
        from_server
    );
    Ok(())
}

///  This is a front-end transparent proxy that forwards all requests to 127.0.0.1:8888 to the destination 127.0.0.1:20172.
///
///  Essentially, this is a simple port forwarding proxy.
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:15432").await?;
    LOG::info!("Listening proxying on 127.0.0.1:15432");
    loop {
        let (inbound, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = transfer(inbound, "127.0.0.1:5432").await {
                LOG::error!("Transfer error: {:?}", e);
            }
        });
    }
}
