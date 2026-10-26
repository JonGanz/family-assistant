//! IMAP client over `async-imap` (tokio runtime), rustls via `webpki-roots`.
//!
//! Mail content is private: nothing here logs bodies, subjects, addresses or credentials.
//!
//! # Integration tests
//! The tests in `tests/imap.rs` need a throwaway GreenMail server (plain IMAP on 3143, IMAPS on
//! 3993) and are `#[ignore]`d unless `FA_TEST_GREENMAIL=host:imap_port:imaps_port` is set:
//!
//! ```text
//! docker run -d --name fa-t02-greenmail -p 33143:3143 -p 33993:3993 \
//!   -e GREENMAIL_OPTS='-Dgreenmail.setup.test.imap -Dgreenmail.setup.test.imaps \
//!   -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.auth.disabled' greenmail/standalone:2.1.8
//! FA_TEST_GREENMAIL=127.0.0.1:33143:33993 cargo test -p fa-connectors --test imap
//! docker rm -f -v fa-t02-greenmail
//! ```

use std::{fmt, sync::Arc, time::Duration};

use async_imap::{
    Client, Session,
    imap_proto::{AttributeValue, NameAttribute, Response, Status},
};
use chrono::{DateTime, NaiveDate, Utc};
use futures::TryStreamExt;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    time::timeout,
};
use tokio_rustls::{
    TlsConnector,
    rustls::{
        self, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        crypto::CryptoProvider,
        pki_types::{CertificateDer, ServerName, UnixTime},
    },
};

use crate::{ConnectorError, error::Result};

const FETCH_BATCH: usize = 50;
const IO_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImapSecurity {
    /// TLS from the first byte (port 993).
    Implicit,
    /// `STARTTLS` upgrade (port 143).
    StartTls,
    /// No encryption. Callers must only pass this when the household allows it.
    Plain,
}

#[derive(Clone)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub security: ImapSecurity,
    pub username: String,
    pub password: String,
    /// Accept any certificate. Tests only.
    pub insecure_tls: bool,
}

impl fmt::Debug for ImapConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImapConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("security", &self.security)
            .field("insecure_tls", &self.insecure_tls)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Caps {
    pub gmail_ext: bool,
    pub move_: bool,
    pub condstore: bool,
    pub uidplus: bool,
    pub idle: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialUse {
    All,
    Archive,
    Junk,
    Trash,
    Sent,
    Drafts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub special_use: Option<SpecialUse>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderStatus {
    pub uidvalidity: u32,
    pub uidnext: u32,
    pub highestmodseq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImapMessage {
    pub uid: u32,
    pub raw: Vec<u8>,
    /// Flags as sent by the server, e.g. `\Seen`.
    pub flags: Vec<String>,
    pub internal_date: Option<DateTime<Utc>>,
    pub gm_msgid: Option<u64>,
    pub gm_thrid: Option<u64>,
    pub gm_labels: Vec<String>,
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send + fmt::Debug {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + fmt::Debug> Io for T {}
type Stream = Box<dyn Io>;

pub struct ImapClient {
    session: Session<Stream>,
}

fn err<E: fmt::Display>(e: E) -> ConnectorError {
    ConnectorError::Imap(e.to_string())
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::aws_lc_rs::default_provider())
}

#[derive(Debug)]
struct AcceptAny(Arc<CryptoProvider>);

impl ServerCertVerifier for AcceptAny {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn connector(insecure: bool) -> Result<TlsConnector> {
    let p = provider();
    let builder = ClientConfig::builder_with_provider(p.clone())
        .with_safe_default_protocol_versions()
        .map_err(err)?;
    let cfg = if insecure {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAny(p)))
            .with_no_client_auth()
    } else {
        let roots = RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        builder.with_root_certificates(roots).with_no_client_auth()
    };
    Ok(TlsConnector::from(Arc::new(cfg)))
}

async fn tls(tcp: TcpStream, host: &str, insecure: bool) -> Result<Stream> {
    let name = ServerName::try_from(host.to_owned()).map_err(err)?;
    let s = connector(insecure)?
        .connect(name, tcp)
        .await
        .map_err(|e| ConnectorError::Network(format!("tls: {e}")))?;
    Ok(Box::new(s))
}

/// Rejects CR, LF and NUL, then quotes for an IMAP command.
fn quote(s: &str) -> Result<String> {
    if s.contains(['\r', '\n', '\0']) {
        return Err(ConnectorError::Imap("invalid character in argument".into()));
    }
    Ok(format!(
        "\"{}\"",
        s.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

fn uid_set(uids: &[u32]) -> String {
    uids.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    // RFC 3501 date-time, the day may be space-padded.
    DateTime::parse_from_str(s.trim(), "%d-%b-%Y %H:%M:%S %z")
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

impl ImapClient {
    pub async fn connect(cfg: ImapConfig) -> Result<Self> {
        timeout(Duration::from_secs(30), Self::connect_inner(cfg))
            .await
            .map_err(|_| ConnectorError::Network("imap connect timed out".into()))?
    }

    async fn connect_inner(cfg: ImapConfig) -> Result<Self> {
        let tcp = TcpStream::connect((cfg.host.as_str(), cfg.port))
            .await
            .map_err(|e| ConnectorError::Network(format!("imap connect: {e}")))?;
        let mut client = match cfg.security {
            ImapSecurity::Implicit => Client::new(tls(tcp, &cfg.host, cfg.insecure_tls).await?),
            ImapSecurity::Plain => Client::new(Box::new(tcp) as Stream),
            ImapSecurity::StartTls => {
                let mut c = Client::new(tcp);
                c.read_response().await.map_err(err)?;
                c.run_command_and_check_ok("STARTTLS", None)
                    .await
                    .map_err(err)?;
                Client::new(tls(c.into_inner(), &cfg.host, cfg.insecure_tls).await?)
            }
        };
        if cfg.security != ImapSecurity::StartTls {
            client.read_response().await.map_err(err)?;
        }
        let session =
            client
                .login(&cfg.username, &cfg.password)
                .await
                .map_err(|(e, _)| match e {
                    async_imap::error::Error::No(_) => ConnectorError::AuthFailed,
                    e => err(e),
                })?;
        Ok(Self { session })
    }

    /// Runs a command, calling `f` for every untagged response until the tagged completion.
    async fn run(&mut self, cmd: &str, mut f: impl FnMut(&Response<'_>)) -> Result<()> {
        let id = self.session.run_command(cmd).await.map_err(err)?;
        loop {
            let resp = timeout(IO_TIMEOUT, self.session.read_response())
                .await
                .map_err(|_| ConnectorError::Network("imap read timed out".into()))?
                .map_err(err)?
                .ok_or_else(|| ConnectorError::Network("imap connection closed".into()))?;
            match resp.parsed() {
                Response::Done { tag, status, .. } if *tag == id => {
                    return match status {
                        Status::Ok => Ok(()),
                        Status::No => Err(ConnectorError::Imap("command rejected (NO)".into())),
                        _ => Err(ConnectorError::Imap("command failed".into())),
                    };
                }
                other => f(other),
            }
        }
    }

    pub async fn capabilities(&mut self) -> Result<Caps> {
        let c = self.session.capabilities().await.map_err(err)?;
        Ok(Caps {
            gmail_ext: c.has_str("X-GM-EXT-1"),
            move_: c.has_str("MOVE"),
            condstore: c.has_str("CONDSTORE") || c.has_str("QRESYNC"),
            uidplus: c.has_str("UIDPLUS"),
            idle: c.has_str("IDLE"),
        })
    }

    pub async fn list_folders(&mut self) -> Result<Vec<Folder>> {
        let names: Vec<_> = self
            .session
            .list(Some(""), Some("*"))
            .await
            .map_err(err)?
            .try_collect()
            .await
            .map_err(err)?;
        Ok(names
            .iter()
            .filter(|n| !n.attributes().contains(&NameAttribute::NoSelect))
            .map(|n| Folder {
                name: n.name().to_owned(),
                special_use: n.attributes().iter().find_map(|a| match a {
                    NameAttribute::All => Some(SpecialUse::All),
                    NameAttribute::Archive => Some(SpecialUse::Archive),
                    NameAttribute::Junk => Some(SpecialUse::Junk),
                    NameAttribute::Trash => Some(SpecialUse::Trash),
                    NameAttribute::Sent => Some(SpecialUse::Sent),
                    NameAttribute::Drafts => Some(SpecialUse::Drafts),
                    _ => None,
                }),
            })
            .collect())
    }

    pub async fn select(&mut self, folder: &str) -> Result<FolderStatus> {
        let condstore = self.capabilities().await?.condstore;
        let mb = if condstore {
            self.session.select_condstore(folder).await
        } else {
            self.session.select(folder).await
        }
        .map_err(err)?;
        Ok(FolderStatus {
            uidvalidity: mb
                .uid_validity
                .ok_or_else(|| ConnectorError::Imap("no UIDVALIDITY".into()))?,
            uidnext: mb
                .uid_next
                .ok_or_else(|| ConnectorError::Imap("no UIDNEXT".into()))?,
            highestmodseq: mb.highest_modseq,
        })
    }

    /// `UID SEARCH SINCE <date>`, plus `X-GM-RAW "<query>"` on Gmail-extended servers.
    pub async fn uid_search_since(
        &mut self,
        date: NaiveDate,
        gm_raw: Option<&str>,
    ) -> Result<Vec<u32>> {
        let mut q = format!("SINCE {}", date.format("%d-%b-%Y"));
        if let Some(raw) = gm_raw.filter(|s| !s.trim().is_empty()) {
            q.push_str(&format!(" X-GM-RAW {}", quote(raw)?));
        }
        let mut uids: Vec<u32> = self
            .session
            .uid_search(q)
            .await
            .map_err(err)?
            .into_iter()
            .collect();
        uids.sort_unstable();
        Ok(uids)
    }

    /// Fetches in batches of 50 with `BODY.PEEK[]`, so nothing is marked read.
    pub async fn uid_fetch(&mut self, uids: &[u32], with_gmail: bool) -> Result<Vec<ImapMessage>> {
        let items = if with_gmail {
            "(UID FLAGS INTERNALDATE BODY.PEEK[] X-GM-MSGID X-GM-THRID X-GM-LABELS)"
        } else {
            "(UID FLAGS INTERNALDATE BODY.PEEK[])"
        };
        let mut out = Vec::with_capacity(uids.len());
        for chunk in uids.chunks(FETCH_BATCH) {
            let cmd = format!("UID FETCH {} {items}", uid_set(chunk));
            self.run(&cmd, |r| {
                if let Some(m) = message_of(r) {
                    out.push(m);
                }
            })
            .await?;
        }
        Ok(out)
    }

    /// Messages with UID >= `from_uid` (incremental sync from the stored `UIDNEXT`).
    pub async fn uid_fetch_range(
        &mut self,
        from_uid: u32,
        with_gmail: bool,
    ) -> Result<Vec<ImapMessage>> {
        // `n:*` always matches the last message even when its UID is below `n`.
        let mut uids: Vec<u32> = self
            .session
            .uid_search(format!("UID {from_uid}:*"))
            .await
            .map_err(err)?
            .into_iter()
            .filter(|u| *u >= from_uid)
            .collect();
        uids.sort_unstable();
        self.uid_fetch(&uids, with_gmail).await
    }

    /// Gmail labels of messages changed after `modseq` (needs `X-GM-EXT-1` and `CONDSTORE`).
    pub async fn labels_changed_since(&mut self, modseq: u64) -> Result<Vec<(u32, Vec<String>)>> {
        let mut out = Vec::new();
        let cmd = format!("UID FETCH 1:* (UID X-GM-LABELS) (CHANGEDSINCE {modseq})");
        self.run(&cmd, |r| {
            if let Some(m) = message_of(r) {
                out.push((m.uid, m.gm_labels));
            }
        })
        .await?;
        Ok(out)
    }

    /// `UID MOVE`, else `UID COPY` + `\Deleted` + `UID EXPUNGE` (needs `UIDPLUS`).
    pub async fn uid_move(&mut self, uid: u32, folder: &str) -> Result<()> {
        let caps = self.capabilities().await?;
        let dest = quote(folder)?;
        if caps.move_ {
            return self.run(&format!("UID MOVE {uid} {dest}"), |_| {}).await;
        }
        if !caps.uidplus {
            return Err(ConnectorError::Imap(
                "server supports neither MOVE nor UIDPLUS".into(),
            ));
        }
        self.run(&format!("UID COPY {uid} {dest}"), |_| {}).await?;
        self.run(
            &format!("UID STORE {uid} +FLAGS.SILENT (\\Deleted)"),
            |_| {},
        )
        .await?;
        self.run(&format!("UID EXPUNGE {uid}"), |_| {}).await
    }

    /// Adds or removes system flags such as `\Seen` or `\Flagged`.
    pub async fn uid_store_flags(&mut self, uid: u32, add: bool, flags: &[&str]) -> Result<()> {
        if flags.is_empty() {
            return Ok(());
        }
        if flags.iter().any(|f| {
            f.is_empty()
                || !f
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "\\$_-".contains(c))
        }) {
            return Err(ConnectorError::Imap("invalid flag".into()));
        }
        let sign = if add { '+' } else { '-' };
        self.run(
            &format!("UID STORE {uid} {sign}FLAGS.SILENT ({})", flags.join(" ")),
            |_| {},
        )
        .await
    }

    /// Adds or removes Gmail labels (`X-GM-LABELS`).
    pub async fn gm_labels(&mut self, uid: u32, add: bool, labels: &[&str]) -> Result<()> {
        if labels.is_empty() {
            return Ok(());
        }
        let list = labels
            .iter()
            .map(|l| quote(l))
            .collect::<Result<Vec<_>>>()?
            .join(" ");
        let sign = if add { '+' } else { '-' };
        self.run(
            &format!("UID STORE {uid} {sign}X-GM-LABELS.SILENT ({list})"),
            |_| {},
        )
        .await
    }

    pub async fn logout(&mut self) -> Result<()> {
        self.session.logout().await.map_err(err)
    }
}

/// Builds a message from a FETCH response; `None` for responses without UID or body.
fn message_of(r: &Response<'_>) -> Option<ImapMessage> {
    let Response::Fetch(_, attrs) = r else {
        return None;
    };
    let mut m = ImapMessage {
        uid: 0,
        raw: Vec::new(),
        flags: Vec::new(),
        internal_date: None,
        gm_msgid: None,
        gm_thrid: None,
        gm_labels: Vec::new(),
    };
    let mut has_body = false;
    for a in attrs {
        match a {
            AttributeValue::Uid(u) => m.uid = *u,
            AttributeValue::Flags(f) => m.flags = f.iter().map(|s| s.to_string()).collect(),
            AttributeValue::InternalDate(d) => m.internal_date = parse_date(d),
            AttributeValue::BodySection {
                data: Some(d),
                section: None,
                ..
            } => {
                m.raw = d.to_vec();
                has_body = true;
            }
            AttributeValue::GmailMsgId(id) => m.gm_msgid = Some(*id),
            AttributeValue::GmailThrId(id) => m.gm_thrid = Some(*id),
            AttributeValue::GmailLabels(l) => {
                m.gm_labels = l.iter().map(|s| s.to_string()).collect()
            }
            _ => {}
        }
    }
    // Label-only fetches carry no body.
    (m.uid != 0 && (has_body || !m.gm_labels.is_empty() || m.gm_msgid.is_some())).then_some(m)
}
