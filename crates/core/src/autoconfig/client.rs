//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::proto::rr::RData;
use hickory_resolver::TokioResolver;
use quick_xml::de::from_str;
use reqwest::Client;
use serde::Deserialize;

use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::raise_error;

/// Parsed result from Thunderbird-style autoconfig XML or DNS SRV fallback.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailConfig {
    pub incoming: Vec<IncomingServer>,
    pub outgoing: Vec<OutgoingServer>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct IncomingServer {
    #[serde(rename = "@type")]
    pub protocol: String,
    pub hostname: String,
    #[serde(default)]
    pub port: u16,
    #[serde(rename = "socketType")]
    pub socket_type: String,
    pub username: String,
    /// Authentication method from the XML, e.g. "OAuth2", "password-cleartext",
    /// "password-encrypted", "GSSAPI", "NTLM".  Absent in DNS SRV fallback.
    #[serde(default)]
    pub authentication: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct OutgoingServer {
    #[serde(rename = "@type")]
    pub protocol: String,
    pub hostname: String,
    #[serde(default)]
    pub port: u16,
    #[serde(rename = "socketType")]
    pub socket_type: String,
    pub username: String,
}

// ---------------------------------------------------------------------------
// Internal XML wrapper structs matching the Thunderbird config-v1.1 schema:
// <clientConfig> → <emailProvider> → <incomingServer> / <outgoingServer>
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename = "clientConfig")]
struct ClientConfig {
    #[serde(rename = "emailProvider", default)]
    email_providers: Vec<EmailProvider>,
}

#[derive(Debug, Deserialize)]
struct EmailProvider {
    #[serde(rename = "incomingServer", default)]
    incoming_servers: Vec<IncomingServer>,
    #[serde(rename = "outgoingServer", default)]
    outgoing_servers: Vec<OutgoingServer>,
}

/// Parse Thunderbird autoconfig XML into a `MailConfig`.
/// Exposed for unit testing.
pub(crate) fn parse_autoconfig_xml(xml: &str) -> Option<MailConfig> {
    let client_config: ClientConfig = from_str(xml).ok()?;
    let provider = client_config.email_providers.into_iter().next()?;
    Some(MailConfig {
        incoming: provider.incoming_servers,
        outgoing: provider.outgoing_servers,
    })
}

// ---------------------------------------------------------------------------
// Network helpers
// ---------------------------------------------------------------------------

async fn fetch_xml(client: &Client, url: &str) -> Option<MailConfig> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let text = resp.text().await.ok()?;
    parse_autoconfig_xml(&text)
}

async fn lookup_srv(domain: &str) -> Option<MailConfig> {
    let resolver = TokioResolver::builder(TokioRuntimeProvider::default())
        .ok()?
        .build()
        .ok()?;

    let imap_srv = format!("_imaps._tcp.{}.", domain);
    let imap_lookup = resolver.srv_lookup(imap_srv).await.ok()?;
    let imap_record = imap_lookup.answers().first()?;
    let (imap_host, imap_port) = match &imap_record.data {
        RData::SRV(srv) => {
            let host = srv.target.to_string().trim_end_matches('.').to_string();
            (host, srv.port)
        }
        _ => return None,
    };

    let smtp_srv = format!("_submission._tcp.{}.", domain);
    let smtp_lookup = resolver.srv_lookup(smtp_srv).await.ok()?;
    let smtp_record = smtp_lookup.answers().first()?;
    let (smtp_host, smtp_port) = match &smtp_record.data {
        RData::SRV(srv) => {
            let host = srv.target.to_string().trim_end_matches('.').to_string();
            (host, srv.port)
        }
        _ => return None,
    };

    Some(MailConfig {
        incoming: vec![IncomingServer {
            protocol: "imap".to_string(),
            hostname: imap_host,
            port: imap_port,
            socket_type: "SSL".to_string(),
            username: "%EMAILADDRESS%".to_string(),
            authentication: String::new(),
        }],
        outgoing: vec![OutgoingServer {
            protocol: "smtp".to_string(),
            hostname: smtp_host,
            port: smtp_port,
            socket_type: "STARTTLS".to_string(),
            username: "%EMAILADDRESS%".to_string(),
        }],
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Discover mail server configuration for a domain using the Thunderbird
/// autoconfig protocol (ISPDB), DNS SRV, MX fallback, and finally guessing.
///
/// Probe order:
/// 1. `https://autoconfig.{domain}/mail/config-v1.1.xml`
/// 2. `http://autoconfig.{domain}/mail/config-v1.1.xml`
/// 3. `https://{domain}/.well-known/autoconfig/mail/config-v1.1.xml`
/// 4. `http://{domain}/.well-known/autoconfig/mail/config-v1.1.xml`
/// 5. DNS SRV records (`_imaps._tcp` / `_submission._tcp`)
/// 6. Thunderbird central ISPDB (`https://autoconfig.thunderbird.net/v1.1/{domain}`)
/// 7. MX lookup → ISPDB for MX domain
/// 8. MX lookup → ISP autoconfig for MX domain
/// 9. GuessConfig — probe common hostnames + ports
pub async fn fetch(domain: &str) -> BichonResult<MailConfig> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    // ── ISP autoconfig (HTTPS, then HTTP) ──────────────────────────
    if let Some(config) = fetch_xml(
        &client,
        &format!("https://autoconfig.{domain}/mail/config-v1.1.xml"),
    )
    .await
    {
        return Ok(config);
    }
    if let Some(config) = fetch_xml(
        &client,
        &format!("http://autoconfig.{domain}/mail/config-v1.1.xml"),
    )
    .await
    {
        return Ok(config);
    }

    // ── Well-known path (HTTPS, then HTTP) ─────────────────────────
    if let Some(config) = fetch_xml(
        &client,
        &format!("https://{domain}/.well-known/autoconfig/mail/config-v1.1.xml"),
    )
    .await
    {
        return Ok(config);
    }
    if let Some(config) = fetch_xml(
        &client,
        &format!("http://{domain}/.well-known/autoconfig/mail/config-v1.1.xml"),
    )
    .await
    {
        return Ok(config);
    }

    // ── DNS SRV records ────────────────────────────────────────────
    if let Some(config) = lookup_srv(domain).await {
        return Ok(config);
    }

    // ── Thunderbird central ISPDB ──────────────────────────────────
    if let Some(config) = fetch_xml(
        &client,
        &format!("https://autoconfig.thunderbird.net/v1.1/{domain}"),
    )
    .await
    {
        return Ok(config);
    }

    // ── MX fallback ────────────────────────────────────────────────
    if let Some(config) = fetch_for_mx(&client, domain).await {
        return Ok(config);
    }

    // ── GuessConfig ────────────────────────────────────────────────
    if let Some(config) = crate::autoconfig::guess::guess_config(domain).await {
        return Ok(config);
    }

    Err(raise_error!(
        format!("No autoconfig found for domain: {domain}"),
        ErrorCode::InternalError
    ))
}

/// DNS MX lookup → retry ISPDB and ISP autoconfig for the MX domain.
///
/// Many self-hosted domains have their MX pointed at Google, Microsoft, etc.
/// The MX domain's ISPDB entry covers the original domain.
async fn fetch_for_mx(client: &Client, domain: &str) -> Option<MailConfig> {
    let mx_domain = lookup_mx_domain(domain).await?;
    if mx_domain == domain.to_ascii_lowercase() {
        return None; // same domain, already tried above
    }

    // Try ISPDB for the MX domain
    if let Some(config) = fetch_xml(
        client,
        &format!("https://autoconfig.thunderbird.net/v1.1/{mx_domain}"),
    )
    .await
    {
        return Some(config);
    }

    // Try ISP autoconfig for the MX domain (HTTPS then HTTP)
    if let Some(config) = fetch_xml(
        client,
        &format!("https://autoconfig.{mx_domain}/mail/config-v1.1.xml"),
    )
    .await
    {
        return Some(config);
    }
    if let Some(config) = fetch_xml(
        client,
        &format!("http://autoconfig.{mx_domain}/mail/config-v1.1.xml"),
    )
    .await
    {
        return Some(config);
    }

    None
}

/// DNS MX lookup → extract the second-level domain of the first MX hostname.
async fn lookup_mx_domain(domain: &str) -> Option<String> {
    let resolver = TokioResolver::builder(TokioRuntimeProvider::default())
        .ok()?
        .build()
        .ok()?;
    let lookup = resolver.mx_lookup(domain).await.ok()?;
    let record = lookup.answers().first()?;
    let mx_host = match &record.data {
        RData::MX(mx) => mx.exchange.to_string().trim_end_matches('.').to_string(),
        _ => return None,
    };

    // Extract a reasonable base domain from the MX hostname.
    // E.g., "aspmx.l.google.com" → "google.com"
    //       "company.mail.protection.outlook.com" → "outlook.com"
    extract_base_domain(&mx_host)
}

/// Extract the top two labels from a hostname as a rough base domain.
fn extract_base_domain(host: &str) -> Option<String> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 2 {
        Some(parts[parts.len() - 2..].join("."))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_valid_domain() {
        let domains = vec![
            // North America
            ("gmail.com", "Google Gmail"),
            ("outlook.com", "Microsoft Outlook"),
            ("hotmail.com", "Microsoft Hotmail"),
            ("yahoo.com", "Yahoo Mail"),
            ("icloud.com", "Apple iCloud"),
            ("aol.com", "AOL Mail"),
            ("protonmail.com", "ProtonMail"),
            ("zoho.com", "Zoho Mail"),
            ("fastmail.com", "FastMail"),
            // Europe
            ("gmx.de", "GMX Germany"),
            ("gmx.net", "GMX International"),
            ("web.de", "Web.de Germany"),
            ("freenet.de", "Freenet Germany"),
            ("mail.ru", "Mail.ru Russia"),
            ("yandex.ru", "Yandex Russia"),
            ("orange.fr", "Orange France"),
            ("laposte.net", "La Poste France"),
            ("libero.it", "Libero Italy"),
            ("tiscali.it", "Tiscali Italy"),
            ("telenet.be", "Telenet Belgium"),
            // Asia Pacific
            ("qq.com", "Tencent QQ"),
            ("163.com", "NetEase 163"),
            ("126.com", "NetEase 126"),
            ("sina.com", "Sina Mail"),
            ("naver.com", "Naver Korea"),
        ];

        for (domain, label) in &domains {
            let result = fetch(domain).await;
            match result {
                Ok(config) => println!("✅ [{label}] {domain}: {config:#?}"),
                Err(e) => println!("⚠️  [{label}] {domain}: {e:?}"),
            }
        }
    }

    #[test]
    fn test_parse_autoconfig_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<clientConfig version="1.1">
    <emailProvider id="example.com">
        <domain>example.com</domain>
        <incomingServer type="imap">
            <hostname>imap.example.com</hostname>
            <port>993</port>
            <socketType>SSL</socketType>
            <username>%EMAILADDRESS%</username>
            <authentication>password-cleartext</authentication>
        </incomingServer>
        <outgoingServer type="smtp">
            <hostname>smtp.example.com</hostname>
            <port>587</port>
            <socketType>STARTTLS</socketType>
            <username>%EMAILADDRESS%</username>
        </outgoingServer>
    </emailProvider>
</clientConfig>"#;

        let config = parse_autoconfig_xml(xml).expect("should parse valid XML");
        assert_eq!(config.incoming.len(), 1);
        assert_eq!(config.incoming[0].protocol, "imap");
        assert_eq!(config.incoming[0].hostname, "imap.example.com");
        assert_eq!(config.incoming[0].port, 993);
        assert_eq!(config.incoming[0].socket_type, "SSL");
        assert_eq!(config.incoming[0].username, "%EMAILADDRESS%");
        assert_eq!(config.incoming[0].authentication, "password-cleartext");

        assert_eq!(config.outgoing.len(), 1);
        assert_eq!(config.outgoing[0].protocol, "smtp");
        assert_eq!(config.outgoing[0].hostname, "smtp.example.com");
        assert_eq!(config.outgoing[0].port, 587);
        assert_eq!(config.outgoing[0].socket_type, "STARTTLS");
        assert_eq!(config.outgoing[0].username, "%EMAILADDRESS%");
    }

    #[test]
    fn test_parse_autoconfig_xml_invalid() {
        assert!(parse_autoconfig_xml("not xml").is_none());
        assert!(parse_autoconfig_xml("<clientConfig></clientConfig>").is_none());
    }

    #[test]
    fn test_extract_base_domain() {
        assert_eq!(
            extract_base_domain("aspmx.l.google.com"),
            Some("google.com".to_string())
        );
        assert_eq!(
            extract_base_domain("company.mail.protection.outlook.com"),
            Some("outlook.com".to_string())
        );
        assert_eq!(
            extract_base_domain("mx.example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_base_domain("example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(extract_base_domain("localhost"), None);
    }

    #[tokio::test]
    async fn test_lookup_srv_gmail() {
        // Gmail should have SRV records for IMAPS and SMTP submission
        let config = lookup_srv("gmail.com").await;
        assert!(config.is_some(), "Gmail should have SRV records");
        let config = config.unwrap();
        assert_eq!(config.incoming.len(), 1);
        assert_eq!(config.incoming[0].protocol, "imap");
        assert_eq!(config.incoming[0].socket_type, "SSL");
        assert!(!config.incoming[0].hostname.is_empty());
        assert!(config.incoming[0].port > 0);
        assert_eq!(config.outgoing.len(), 1);
        assert_eq!(config.outgoing[0].protocol, "smtp");
        assert_eq!(config.outgoing[0].socket_type, "STARTTLS");
        assert!(!config.outgoing[0].hostname.is_empty());
        assert!(config.outgoing[0].port > 0);
    }

    #[tokio::test]
    async fn test_lookup_srv_nonexistent() {
        // A domain without SRV records should return None
        let config = lookup_srv("this-domain-definitely-does-not-exist-12345.com").await;
        assert!(config.is_none());
    }
}
