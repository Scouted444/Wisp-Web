use std::sync::Arc;

use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::{Certificate, ClientConfig, PrivateKey, ServerConfig, ServerName};

/// Generates a fresh self-signed cert for `hostnames`, valid for this process's
/// lifetime only. Fine for local dev/testing; a real deployment needs a real CA
/// (this is the gap Gurted's "GurtCA" fills).
pub fn generate_self_signed(hostnames: Vec<String>) -> (Certificate, PrivateKey) {
    let cert = rcgen::generate_simple_self_signed(hostnames).expect("cert generation failed");
    let cert_der = cert.serialize_der().expect("cert serialization failed");
    let key_der = cert.serialize_private_key_der();
    (Certificate(cert_der), PrivateKey(key_der))
}

pub fn server_config(cert: Certificate, key: PrivateKey) -> Arc<ServerConfig> {
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .expect("bad cert/key for server config");
    Arc::new(config)
}

/// Accepts ANY server certificate without checking it against a CA. This is
/// deliberately insecure and exists only so a local client can talk to a
/// local server using a self-signed cert without extra setup. Real Wisp
/// deployments need proper certificate validation here.
struct AcceptAnyCert;

impl ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}

pub fn client_config_insecure() -> Arc<ClientConfig> {
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCert))
        .with_no_client_auth();
    Arc::new(config)
}
