//! rustls client session for outbound TLS (Phase 51 / ADR-007).
//!
//! Host tests use rustls+ring. The seL4 PD target uses rustls-rustcrypto and a
//! custom `getrandom`. Apps never see this crate — they speak [`TlsRequest`].

#![no_std]

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use core::time::Duration;

use rustls::{
    client::UnbufferedClientConnection,
    pki_types::{CertificateDer, ServerName, UnixTime},
    unbuffered::{ConnectionState, EncodeError, UnbufferedStatus},
    ClientConfig, RootCertStore,
};

/// Baked-in smoke CA (SAN `host` / `10.0.2.2`). Not a production root store.
pub const SMOKE_CA_DER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../support/tls/lerux-smoke-ca.der"
));

/// Fixed unix time inside the smoke-cert validity window (2026-08-15–2036).
const SMOKE_UNIX_TIME: u64 = 1_790_000_000;

const OUT_BUF: usize = 16_640;

/// What the caller should do after [`ClientSession::drive`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    WantSend,
    WantRecv,
    Connected,
    Closed,
}

/// Opaque rustls / name-parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsError;

/// One outbound TLS 1.2/1.3 client.
pub struct ClientSession {
    conn: UnbufferedClientConnection,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
    plaintext: Vec<u8>,
    connected: bool,
}

impl ClientSession {
    /// Start a client toward `server_name` (SNI), trusting the smoke CA.
    pub fn new(server_name: &str) -> Result<Self, TlsError> {
        #[cfg(target_os = "none")]
        install_rng();

        let config = client_config()?;
        let name = ServerName::try_from(server_name)
            .map_err(|_| TlsError)?
            .to_owned();
        let conn = UnbufferedClientConnection::new(config, name).map_err(|_| TlsError)?;
        Ok(Self {
            conn,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            plaintext: Vec::new(),
            connected: false,
        })
    }

    pub fn feed_incoming(&mut self, data: &[u8]) {
        self.incoming.extend_from_slice(data);
    }

    pub fn take_outgoing(&mut self) -> Vec<u8> {
        core::mem::take(&mut self.outgoing)
    }

    pub fn has_outgoing(&self) -> bool {
        !self.outgoing.is_empty()
    }

    pub fn read_plain(&mut self, buf: &mut [u8]) -> usize {
        let n = self.plaintext.len().min(buf.len());
        buf[..n].copy_from_slice(&self.plaintext[..n]);
        self.plaintext.drain(..n);
        n
    }

    /// Encrypt application bytes into the outgoing TLS buffer.
    pub fn write_plain(&mut self, data: &[u8]) -> Result<(), TlsError> {
        let UnbufferedStatus { discard, state } = self.conn.process_tls_records(&mut self.incoming);
        let result = match state.map_err(|_| TlsError)? {
            ConnectionState::WriteTraffic(mut wt) => {
                encrypt_into(&mut wt, data, &mut self.outgoing)
            }
            ConnectionState::TransmitTlsData(mut ttd) => {
                if let Some(mut wt) = ttd.may_encrypt_app_data() {
                    encrypt_into(&mut wt, data, &mut self.outgoing)
                } else {
                    ttd.done();
                    Err(TlsError)
                }
            }
            _ => Err(TlsError),
        };
        if discard > 0 {
            self.incoming.drain(..discard);
        }
        result
    }

    /// Advance handshake or drain readable records.
    pub fn drive(&mut self) -> Result<Status, TlsError> {
        for _ in 0..16 {
            let UnbufferedStatus { mut discard, state } =
                self.conn.process_tls_records(&mut self.incoming);
            let status = match state.map_err(|_| TlsError)? {
                ConnectionState::EncodeTlsData(mut etd) => {
                    let mut buf = [0u8; OUT_BUF];
                    match etd.encode(&mut buf) {
                        Ok(n) => self.outgoing.extend_from_slice(&buf[..n]),
                        Err(EncodeError::AlreadyEncoded) => {}
                        Err(_) => {
                            if discard > 0 {
                                self.incoming.drain(..discard);
                            }
                            return Err(TlsError);
                        }
                    }
                    if self.outgoing.is_empty() {
                        None
                    } else {
                        Some(Status::WantSend)
                    }
                }
                ConnectionState::TransmitTlsData(ttd) => {
                    ttd.done();
                    if self.outgoing.is_empty() {
                        None
                    } else {
                        Some(Status::WantSend)
                    }
                }
                ConnectionState::BlockedHandshake => Some(Status::WantRecv),
                ConnectionState::WriteTraffic(_) => {
                    self.connected = true;
                    Some(Status::Connected)
                }
                ConnectionState::ReadTraffic(mut rt) => {
                    self.connected = true;
                    let mut extra = 0usize;
                    while let Some(item) = rt.next_record() {
                        let rec = item.map_err(|_| TlsError)?;
                        extra += rec.discard;
                        self.plaintext.extend_from_slice(rec.payload);
                    }
                    discard = discard.saturating_add(extra);
                    Some(Status::Connected)
                }
                ConnectionState::Closed | ConnectionState::PeerClosed => Some(Status::Closed),
                _ => {
                    if self.connected {
                        Some(Status::Connected)
                    } else {
                        Some(Status::WantRecv)
                    }
                }
            };
            if discard > 0 {
                self.incoming.drain(..discard);
            }
            if let Some(s) = status {
                return Ok(s);
            }
        }
        if self.connected {
            Ok(Status::Connected)
        } else if self.outgoing.is_empty() {
            Ok(Status::WantRecv)
        } else {
            Ok(Status::WantSend)
        }
    }
}

fn encrypt_into<Data>(
    wt: &mut rustls::unbuffered::WriteTraffic<'_, Data>,
    data: &[u8],
    outgoing: &mut Vec<u8>,
) -> Result<(), TlsError> {
    let mut buf = [0u8; OUT_BUF];
    let n = wt.encrypt(data, &mut buf).map_err(|_| TlsError)?;
    outgoing.extend_from_slice(&buf[..n]);
    Ok(())
}

fn client_config() -> Result<Arc<ClientConfig>, TlsError> {
    let provider = crypto_provider();
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(SMOKE_CA_DER.to_vec()))
        .map_err(|_| TlsError)?;
    #[cfg(feature = "webpki-roots")]
    {
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    let builder = ClientConfig::builder_with_details(Arc::new(provider), Arc::new(FixedTime));
    let builder = builder
        .with_safe_default_protocol_versions()
        .map_err(|_| TlsError)?;
    Ok(Arc::new(
        builder.with_root_certificates(roots).with_no_client_auth(),
    ))
}

fn crypto_provider() -> rustls::crypto::CryptoProvider {
    #[cfg(target_os = "none")]
    {
        rustls_rustcrypto::provider()
    }
    #[cfg(not(target_os = "none"))]
    {
        rustls::crypto::ring::default_provider()
    }
}

#[derive(Debug)]
struct FixedTime;

impl rustls::time_provider::TimeProvider for FixedTime {
    fn current_time(&self) -> Option<UnixTime> {
        Some(UnixTime::since_unix_epoch(Duration::from_secs(
            SMOKE_UNIX_TIME,
        )))
    }
}

#[cfg(target_os = "none")]
fn install_rng() {
    // `register_custom_getrandom!` is a compile-time hook; calling this keeps
    // the module linked so rustls-rustcrypto's OsRng can fill handshake bytes.
}

#[cfg(target_os = "none")]
mod sel4_rng {
    use core::sync::atomic::{AtomicU64, Ordering};

    static STATE: AtomicU64 = AtomicU64::new(0xC0FF_EE00_D15C_A11E);

    fn fill(buf: &mut [u8]) -> Result<(), getrandom::Error> {
        for chunk in buf.chunks_mut(8) {
            let mut x = STATE.load(Ordering::Relaxed);
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            STATE.store(x, Ordering::Relaxed);
            let bytes = x.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        Ok(())
    }

    getrandom::register_custom_getrandom!(fill);
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use rustls::{pki_types::CertificateDer, ServerConfig, ServerConnection};
    use std::{
        io::{Read, Write},
        vec,
    };

    fn server_config() -> Arc<ServerConfig> {
        let cert_pem = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../support/tls/lerux-smoke-server.pem"
        ));
        let key_pem = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../support/tls/lerux-smoke-server.key"
        ));
        let mut cert_reader = &cert_pem[..];
        let certs: vec::Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
            .map(|c| c.expect("cert"))
            .collect();
        let mut key_reader = &key_pem[..];
        let key = rustls_pemfile::private_key(&mut key_reader)
            .expect("key parse")
            .expect("key");
        Arc::new(
            ServerConfig::builder_with_provider(rustls::crypto::ring::default_provider().into())
                .with_safe_default_protocol_versions()
                .expect("versions")
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .expect("server config"),
        )
    }

    fn pump(client: &mut ClientSession, server: &mut ServerConnection) -> Status {
        for _ in 0..64 {
            let status = client.drive().expect("drive");
            match status {
                Status::WantSend => {
                    let out = client.take_outgoing();
                    if !out.is_empty() {
                        server
                            .read_tls(&mut out.as_slice())
                            .expect("server read_tls");
                        server.process_new_packets().expect("server process");
                    }
                }
                Status::WantRecv => {
                    let mut buf = [0u8; OUT_BUF];
                    match server.write_tls(&mut buf.as_mut_slice()) {
                        Ok(0) => {}
                        Ok(n) => client.feed_incoming(&buf[..n]),
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(e) => panic!("server write_tls: {e}"),
                    }
                }
                Status::Connected | Status::Closed => return status,
            }
        }
        panic!("handshake did not finish")
    }

    #[test]
    fn smoke_ca_handshake_and_appdata() {
        let mut client = ClientSession::new("host").expect("client");
        let mut server = ServerConnection::new(server_config()).expect("server conn");
        assert_eq!(pump(&mut client, &mut server), Status::Connected);

        client.write_plain(b"ping").expect("write_plain");
        let out = client.take_outgoing();
        assert!(!out.is_empty());
        server
            .read_tls(&mut out.as_slice())
            .expect("server app read");
        server.process_new_packets().expect("server app process");
        let mut plain = [0u8; 16];
        let n = server.reader().read(&mut plain).expect("server reader");
        assert_eq!(&plain[..n], b"ping");

        server.writer().write_all(b"pong").expect("server write");
        let mut buf = [0u8; OUT_BUF];
        let n = server
            .write_tls(&mut buf.as_mut_slice())
            .expect("server tls out");
        client.feed_incoming(&buf[..n]);
        assert_eq!(client.drive().expect("drive app"), Status::Connected);
        let n = client.read_plain(&mut plain);
        assert_eq!(&plain[..n], b"pong");
    }
}
