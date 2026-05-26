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

use crate::account::entity::Encryption;
use crate::autoconfig::client::{self, IncomingServer, MailConfig};
use crate::autoconfig::load::{mail_config_to_server_config, socket_type_to_encryption};

// ---------------------------------------------------------------------------
// XML parsing tests
// ---------------------------------------------------------------------------

fn make_valid_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<clientConfig version="1.1">
  <emailProvider id="example.com">
    <domain>example.com</domain>
    <displayName>Example Mail</displayName>
    <incomingServer type="imap">
      <hostname>imap.example.com</hostname>
      <port>993</port>
      <socketType>SSL</socketType>
      <username>%EMAILADDRESS%</username>
    </incomingServer>
    <outgoingServer type="smtp">
      <hostname>smtp.example.com</hostname>
      <port>587</port>
      <socketType>STARTTLS</socketType>
      <username>%EMAILADDRESS%</username>
    </outgoingServer>
  </emailProvider>
</clientConfig>"#
        .to_string()
}

#[test]
fn parse_valid_xml() {
    let xml = make_valid_xml();
    let config = client::parse_autoconfig_xml(&xml).expect("should parse valid XML");

    assert_eq!(config.incoming.len(), 1);
    let imap = &config.incoming[0];
    assert_eq!(imap.protocol, "imap");
    assert_eq!(imap.hostname, "imap.example.com");
    assert_eq!(imap.port, 993);
    assert_eq!(imap.socket_type, "SSL");
    assert_eq!(imap.username, "%EMAILADDRESS%");

    assert_eq!(config.outgoing.len(), 1);
    let smtp = &config.outgoing[0];
    assert_eq!(smtp.protocol, "smtp");
    assert_eq!(smtp.hostname, "smtp.example.com");
    assert_eq!(smtp.port, 587);
    assert_eq!(smtp.socket_type, "STARTTLS");
}

#[test]
fn parse_xml_empty_body() {
    let xml = r#"<?xml version="1.0"?><clientConfig></clientConfig>"#;
    let config = client::parse_autoconfig_xml(xml);
    assert!(config.is_none(), "no emailProvider → None");
}

#[test]
fn parse_xml_no_incoming_servers() {
    let xml = r#"<?xml version="1.0"?>
<clientConfig version="1.1">
  <emailProvider id="example.com">
    <domain>example.com</domain>
  </emailProvider>
</clientConfig>"#;
    let config = client::parse_autoconfig_xml(xml).expect("should parse");
    assert!(config.incoming.is_empty());
    assert!(config.outgoing.is_empty());
}

#[test]
fn parse_xml_garbage() {
    let config = client::parse_autoconfig_xml("not xml at all");
    assert!(config.is_none());
}

#[test]
fn parse_xml_missing_port_defaults_to_zero() {
    let xml = r#"<?xml version="1.0"?>
<clientConfig version="1.1">
  <emailProvider id="example.com">
    <incomingServer type="imap">
      <hostname>imap.example.com</hostname>
      <socketType>SSL</socketType>
      <username>%EMAILADDRESS%</username>
    </incomingServer>
  </emailProvider>
</clientConfig>"#;
    let config = client::parse_autoconfig_xml(xml).expect("should parse");
    assert_eq!(config.incoming[0].port, 0);
}

#[test]
fn parse_xml_multiple_providers_picks_first() {
    let xml = r#"<?xml version="1.0"?>
<clientConfig version="1.1">
  <emailProvider id="first.example.com">
    <incomingServer type="imap">
      <hostname>imap.first.example.com</hostname>
      <port>993</port>
      <socketType>SSL</socketType>
      <username>%EMAILADDRESS%</username>
    </incomingServer>
  </emailProvider>
  <emailProvider id="second.example.com">
    <incomingServer type="imap">
      <hostname>imap.second.example.com</hostname>
      <port>143</port>
      <socketType>STARTTLS</socketType>
      <username>%EMAILADDRESS%</username>
    </incomingServer>
  </emailProvider>
</clientConfig>"#;
    let config = client::parse_autoconfig_xml(xml).expect("should parse");
    assert_eq!(config.incoming[0].hostname, "imap.first.example.com");
}

// ---------------------------------------------------------------------------
// socket_type → Encryption mapping tests
// ---------------------------------------------------------------------------

#[test]
fn encryption_ssl_uppercase() {
    assert_eq!(socket_type_to_encryption("SSL"), Encryption::Ssl);
}

#[test]
fn encryption_ssl_lowercase() {
    assert_eq!(socket_type_to_encryption("ssl"), Encryption::Ssl);
}

#[test]
fn encryption_tls() {
    assert_eq!(socket_type_to_encryption("TLS"), Encryption::Ssl);
}

#[test]
fn encryption_starttls() {
    assert_eq!(socket_type_to_encryption("STARTTLS"), Encryption::StartTls);
}

#[test]
fn encryption_starttls_lowercase() {
    assert_eq!(socket_type_to_encryption("starttls"), Encryption::StartTls);
}

#[test]
fn encryption_starttls_mixed_case() {
    assert_eq!(socket_type_to_encryption("StartTls"), Encryption::StartTls);
}

#[test]
fn encryption_plain() {
    assert_eq!(socket_type_to_encryption("plain"), Encryption::None);
}

#[test]
fn encryption_empty_string() {
    assert_eq!(socket_type_to_encryption(""), Encryption::None);
}

#[test]
fn encryption_unknown_value() {
    assert_eq!(socket_type_to_encryption("WPA2-ENTERPRISE"), Encryption::None);
}

// ---------------------------------------------------------------------------
// MailConfig → MailServerConfig conversion tests
// ---------------------------------------------------------------------------

fn make_imap_server(host: &str, port: u16, socket_type: &str) -> IncomingServer {
    IncomingServer {
        protocol: "imap".to_string(),
        hostname: host.to_string(),
        port,
        socket_type: socket_type.to_string(),
        username: "%EMAILADDRESS%".to_string(),
        authentication: String::new(),
    }
}

#[test]
fn convert_basic_imap_ssl() {
    let config = MailConfig {
        incoming: vec![make_imap_server("imap.example.com", 993, "SSL")],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should convert");
    assert_eq!(result.imap.host, "imap.example.com");
    assert_eq!(result.imap.port, 993);
    assert_eq!(result.imap.encryption, Encryption::Ssl);
    assert!(result.oauth2.is_none());
}

#[test]
fn convert_imap_starttls_with_default_port() {
    let config = MailConfig {
        incoming: vec![make_imap_server("imap.example.com", 0, "STARTTLS")],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should convert");
    assert_eq!(result.imap.port, 143, "default port for STARTTLS → 143");
    assert_eq!(result.imap.encryption, Encryption::StartTls);
}

#[test]
fn convert_imap_ssl_with_default_port() {
    let config = MailConfig {
        incoming: vec![make_imap_server("imap.example.com", 0, "SSL")],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should convert");
    assert_eq!(result.imap.port, 993, "default port for SSL → 993");
}

#[test]
fn convert_no_imap_only_pop3() {
    let config = MailConfig {
        incoming: vec![IncomingServer {
            protocol: "pop3".to_string(),
            hostname: "pop.example.com".to_string(),
            port: 995,
            socket_type: "SSL".to_string(),
            username: "%EMAILADDRESS%".to_string(),
            authentication: String::new(),
        }],
        outgoing: vec![],
    };
    assert!(mail_config_to_server_config(&config).is_none());
}

#[test]
fn convert_empty_incoming() {
    let config = MailConfig {
        incoming: vec![],
        outgoing: vec![],
    };
    assert!(mail_config_to_server_config(&config).is_none());
}

#[test]
fn convert_picks_imap_over_pop3() {
    let config = MailConfig {
        incoming: vec![
            IncomingServer {
                protocol: "pop3".to_string(),
                hostname: "pop.example.com".to_string(),
                port: 995,
                socket_type: "SSL".to_string(),
                username: "%EMAILADDRESS%".to_string(),
                authentication: String::new(),
            },
            make_imap_server("imap.example.com", 993, "SSL"),
        ],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should find IMAP");
    assert_eq!(result.imap.host, "imap.example.com");
}

#[test]
fn convert_imaps_protocol_variant() {
    let config = MailConfig {
        incoming: vec![IncomingServer {
            protocol: "imaps".to_string(),
            hostname: "imap.example.com".to_string(),
            port: 993,
            socket_type: "SSL".to_string(),
            username: "%EMAILADDRESS%".to_string(),
            authentication: String::new(),
        }],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should recognize 'imaps'");
    assert_eq!(result.imap.host, "imap.example.com");
}

#[test]
fn convert_case_insensitive_protocol() {
    let config = MailConfig {
        incoming: vec![IncomingServer {
            protocol: "IMAP".to_string(),
            hostname: "imap.example.com".to_string(),
            port: 143,
            socket_type: "STARTTLS".to_string(),
            username: "%EMAILADDRESS%".to_string(),
            authentication: String::new(),
        }],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should recognize 'IMAP'");
    assert_eq!(result.imap.host, "imap.example.com");
}

#[test]
fn convert_gmail_oauth2() {
    let config = MailConfig {
        incoming: vec![IncomingServer {
            protocol: "imap".to_string(),
            hostname: "imap.gmail.com".to_string(),
            port: 993,
            socket_type: "SSL".to_string(),
            username: "%EMAILADDRESS%".to_string(),
            authentication: "OAuth2".to_string(),
        }],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should convert");
    let oauth2 = result.oauth2.expect("Gmail should have OAuth2");
    assert_eq!(oauth2.issuer, "https://accounts.google.com");
    assert!(oauth2.scope.contains(&"https://mail.google.com/".to_string()));
}

#[test]
fn convert_outlook_oauth2() {
    let config = MailConfig {
        incoming: vec![IncomingServer {
            protocol: "imap".to_string(),
            hostname: "outlook.office365.com".to_string(),
            port: 993,
            socket_type: "SSL".to_string(),
            username: "%EMAILADDRESS%".to_string(),
            authentication: "OAuth2".to_string(),
        }],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should convert");
    let oauth2 = result.oauth2.expect("Outlook should have OAuth2");
    assert!(oauth2.issuer.contains("microsoftonline"));
}

#[test]
fn convert_unknown_host_no_oauth2() {
    // OAuth2 auth flag on an unknown hostname → no OAuth2 returned
    let config = MailConfig {
        incoming: vec![IncomingServer {
            protocol: "imap".to_string(),
            hostname: "mail.random-isp.example".to_string(),
            port: 993,
            socket_type: "SSL".to_string(),
            username: "%EMAILADDRESS%".to_string(),
            authentication: "OAuth2".to_string(),
        }],
        outgoing: vec![],
    };
    let result = mail_config_to_server_config(&config).expect("should convert");
    assert!(result.oauth2.is_none(), "unknown hostname → no OAuth2 mapping");
}
