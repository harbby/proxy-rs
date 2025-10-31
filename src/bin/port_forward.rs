use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::Result;
use tokio::io;
use log as LOG;

async fn transfer(mut inbound: TcpStream, addr: SocketAddr, target: String) {
    let local_port = addr.port();
    let local_host = addr.ip().to_string();

    let mut outbound = match TcpStream::connect(&target).await {
        Ok(outbound) => outbound,
        Err(e) => {
            LOG::error!("connect target {} failed: {}", target, e);
            return;
        }
    };
    LOG::info!("Accepted {} -> {}", addr, target);

    // Method 3
    let (from_client, from_server) = match io::copy_bidirectional(&mut inbound, &mut outbound).await {
        Ok(result) => result,
        Err(e) => {
            LOG::error!("connection actively closed during transmission: {}", e);
            return;
        }
    };
    LOG::info!(
        "proxy accepted tcp:{}:{} -> {target}, transferred {} bytes from client, {} bytes from server",
        local_host, local_port, from_client, from_server
    );
}

async fn bind(port: &str, target: &str) -> Result<impl Future<Output = ()> + use<>> {
    let bind_addr = format!("0.0.0.0:{}", port);
    let target = target.to_string();
    let listener = TcpListener::bind(&bind_addr).await?;
    LOG::info!("Listening on {} -> {}", bind_addr, target);

    let func = async move {
        loop {
            match listener.accept().await {
                Ok((inbound, addr)) => { tokio::spawn(transfer(inbound, addr, target.clone())); }
                Err(e) => LOG::error!("Accept error: {}", e)
            };
        }
    };
    Ok(func)
}

///  ./port_forward 10000=host1:8080 10001=host2:8080
///
///  Essentially, this is a simple port forwarding proxy.
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        LOG::error!("Usage: tcp_proxy <local_port>=<target> [<local_port>=<target> ...]");
        std::process::exit(1);
    }

    let mut handles = Vec::new();
    for arg in args {
        if let Some((port, target)) = arg.split_once('=') {
            let future = bind(port, target).await?;
            handles.push(tokio::spawn(future));
        } else {
            LOG::error!("Invalid argument: {}", arg);
        }
    }

    for handle in handles {
        handle.await?;
    }
    Ok(())
}
