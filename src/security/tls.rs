use std::io;
use std::path::Path;

pub async fn create_tls_acceptor(_cert_path: &Path, _key_path: &Path) -> io::Result<()> {
    // TLS implementation disabled for now - requires rustls version upgrade
    Err(io::Error::new(io::ErrorKind::Unsupported, "TLS not implemented"))
}