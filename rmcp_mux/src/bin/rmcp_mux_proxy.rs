use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use tokio::io::{self, AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::UnixStream;

/// Lightweight STDIO↔Unix-socket proxy for rmcp_mux.
#[derive(Parser, Debug)]
#[command(author, version, about = "Proxy STDIO to an rmcp_mux socket")]
struct ProxyCli {
    /// Path to the Unix socket exposed by rmcp_mux (e.g. ~/mcp-sockets/memory.sock).
    #[arg(long)]
    socket: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = ProxyCli::parse();
    run_proxy(&cli.socket, io::stdin(), io::stdout()).await
}

async fn run_proxy<R, W>(socket: &Path, mut stdin: R, mut stdout: W) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let stream = UnixStream::connect(socket).await?;
    let (mut mux_reader, mut mux_writer) = stream.into_split();

    let to_mux = async {
        io::copy(&mut stdin, &mut mux_writer).await?;
        mux_writer.shutdown().await?;
        Ok::<(), anyhow::Error>(())
    };

    let from_mux = async {
        io::copy(&mut mux_reader, &mut stdout).await?;
        stdout.flush().await?;
        Ok::<(), anyhow::Error>(())
    };

    let _ = tokio::try_join!(to_mux, from_mux)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run_proxy;
    use std::env;
    use std::path::PathBuf;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};
    use tokio::net::UnixListener;

    fn socket_path(name: &str) -> PathBuf {
        let mut p = env::temp_dir();
        p.push(format!("{name}-{}.sock", uuid::Uuid::new_v4()));
        p
    }

    #[tokio::test]
    async fn proxy_forwards_bytes() {
        let path = socket_path("proxy-test");
        let listener = UnixListener::bind(&path).expect("bind socket");

        // Echo server task
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("server accept failed");
            let (mut r, mut w) = stream.split();
            let mut buf = vec![0u8; 256];
            let n = r.read(&mut buf).await.expect("read");
            w.write_all(&buf[..n]).await.expect("write back");
            w.shutdown().await.expect("shutdown");
        });

        // Duplex streams to simulate stdin/stdout
        let (mut stdin_writer, stdin_reader) = duplex(256);
        let (stdout_writer, mut stdout_reader) = duplex(256);

        let proxy_path = path.clone();
        let proxy =
            tokio::spawn(async move { run_proxy(&proxy_path, stdin_reader, stdout_writer).await });

        stdin_writer
            .write_all(b"ping")
            .await
            .expect("write to proxy stdin");
        stdin_writer
            .shutdown()
            .await
            .expect("shutdown stdin writer");

        let mut out = Vec::new();
        stdout_reader.read_to_end(&mut out).await.expect("read out");
        assert_eq!(out, b"ping");

        proxy.await.expect("proxy join").expect("proxy result");
        server.await.expect("server join");
        let _ = std::fs::remove_file(path);
    }
}
