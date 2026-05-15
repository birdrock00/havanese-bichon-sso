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

use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::rr::RData;
use hickory_resolver::proto::rr::RecordType;
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
    let resolver = TokioResolver::builder(TokioConnectionProvider::default())
        .ok()?
        .build();

    let imap_srv = format!("_imaps._tcp.{}.", domain);
    let imap_lookup = resolver.lookup(imap_srv, RecordType::SRV).await.ok()?;
    let imap_record = imap_lookup.iter().next()?;
    let (imap_host, imap_port) = match imap_record {
        RData::SRV(srv) => {
            let host = srv.target().to_string().trim_end_matches('.').to_string();
            (host, srv.port())
        }
        _ => return None,
    };

    let smtp_srv = format!("_submission._tcp.{}.", domain);
    let smtp_lookup = resolver.lookup(smtp_srv, RecordType::SRV).await.ok()?;
    let smtp_record = smtp_lookup.iter().next()?;
    let (smtp_host, smtp_port) = match smtp_record {
        RData::SRV(srv) => {
            let host = srv.target().to_string().trim_end_matches('.').to_string();
            (host, srv.port())
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
/// autoconfig protocol (ISPDB) and DNS SRV fallback.
///
/// Probe order:
/// 1. `https://autoconfig.{domain}/mail/config-v1.1.xml`
/// 2. `https://{domain}/.well-known/autoconfig/mail/config-v1.1.xml`
/// 3. DNS SRV records (`_imaps._tcp` / `_submission._tcp`)
/// 4. Thunderbird central ISPDB (`https://autoconfig.thunderbird.net/v1.1/{domain}`)
pub async fn fetch(domain: &str) -> BichonResult<MailConfig> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    // 1. Try autoconfig subdomain
    if let Some(config) = fetch_xml(
        &client,
        &format!("https://autoconfig.{domain}/mail/config-v1.1.xml"),
    )
    .await
    {
        return Ok(config);
    }

    // 2. Try well-known path
    if let Some(config) = fetch_xml(
        &client,
        &format!("https://{domain}/.well-known/autoconfig/mail/config-v1.1.xml"),
    )
    .await
    {
        return Ok(config);
    }

    // 3. Try DNS SRV records
    if let Some(config) = lookup_srv(domain).await {
        return Ok(config);
    }

    // 4. Fall back to Thunderbird central database
    if let Some(config) = fetch_xml(
        &client,
        &format!("https://autoconfig.thunderbird.net/v1.1/{domain}"),
    )
    .await
    {
        return Ok(config);
    }

    Err(raise_error!(
        format!("No autoconfig found for domain: {domain}"),
        ErrorCode::InternalError
    ))
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
}
