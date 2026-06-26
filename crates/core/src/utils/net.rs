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

use crate::error::code::ErrorCode;
use crate::raise_error;
use crate::settings::proxy::Proxy;
use crate::utils::tls::establish_tls_stream;
use crate::{error::BichonResult, imap::session::SessionStream};
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;
use tokio_socks::tcp::Socks5Stream;
use tracing::error;

pub(crate) const TIMEOUT: Duration = Duration::from_secs(30);

/// Parsed proxy address components.
#[derive(Debug, Clone)]
pub struct ProxyAddr {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub(crate) async fn establish_tcp_connection_with_timeout(
    address: SocketAddr,
    use_proxy: Option<u64>,
) -> BichonResult<Pin<Box<TimeoutStream<TcpStream>>>> {
    // Establish the TCP connection with a timeout
    let tcp_stream = connect_with_optional_proxy(use_proxy, address).await?;
    let mut timeout_stream = TimeoutStream::new(tcp_stream);

    // Set read and write timeouts
    timeout_stream.set_write_timeout(Some(Duration::from_secs(15)));
    timeout_stream.set_read_timeout(Some(Duration::from_secs(30)));

    // Return the timeout-wrapped TCP stream as a Pin
    Ok(Box::pin(timeout_stream))
}

pub async fn establish_tls_connection(
    address: SocketAddr,
    server_hostname: &str,
    alpn_protocols: &[&str],
    use_proxy: Option<u64>,
    dangerous: bool,
) -> BichonResult<impl SessionStream> {
    // Establish the TCP connection with timeout
    let tcp_stream = establish_tcp_connection_with_timeout(address, use_proxy).await?;

    // Wrap the TCP stream with TLS encryption
    let tls_stream =
        establish_tls_stream(server_hostname, alpn_protocols, tcp_stream, dangerous).await?;

    // Return the TLS stream wrapped in a SessionStream
    Ok(tls_stream)
}

/// Parse a proxy URL into its components.
///
/// Supports two formats:
/// - **Standard**: `[scheme://][user:pass@]host:port`
/// - **Non-standard** (some proxy providers): `[scheme://]host:port:username:password`
///
/// The distinguishing feature is the `@` sign in the standard format.
pub fn parse_proxy_url(input: &str) -> BichonResult<ProxyAddr> {
    // Normalize and strip scheme prefix
    let stripped = if let Some(rest) = input
        .strip_prefix("socks5://")
        .or_else(|| input.strip_prefix("SOCKS5://"))
        .or_else(|| input.strip_prefix("Socks5://"))
    {
        rest
    } else if let Some(rest) = input
        .strip_prefix("http://")
        .or_else(|| input.strip_prefix("HTTP://"))
        .or_else(|| input.strip_prefix("Http://"))
    {
        rest
    } else {
        return Err(raise_error!(
            format!(
                "Invalid proxy URL: must start with 'http://' or 'socks5://', got '{}'",
                input
            ),
            ErrorCode::InvalidParameter
        ));
    };

    if stripped.is_empty() {
        return Err(raise_error!(
            "Proxy URL has empty address after scheme.".into(),
            ErrorCode::InvalidParameter
        ));
    }

    // Check for standard format: user:pass@host:port
    if let Some(at_pos) = stripped.rfind('@') {
        let userinfo = &stripped[..at_pos];
        let hostport = &stripped[at_pos + 1..];

        let (username, password) = split_userinfo(userinfo)?;
        let (host, port) = split_hostport(hostport)?;

        return Ok(ProxyAddr {
            host,
            port,
            username,
            password,
        });
    }

    // No '@' — check for non-standard format: host:port:user:pass
    let parts: Vec<&str> = stripped.rsplitn(4, ':').collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>();

    match parts.len() {
        2 => {
            // host:port, no auth
            let (host, port) = split_hostport(stripped)?;
            Ok(ProxyAddr {
                host,
                port,
                username: None,
                password: None,
            })
        }
        4 => {
            // Non-standard: host:port:username:password
            let host = parts[0].to_string();
            let port = parts[1]
                .parse::<u16>()
                .map_err(|_| {
                    raise_error!(
                        format!("Invalid port '{}' in proxy URL.", parts[1]),
                        ErrorCode::InvalidParameter
                    )
                })?;
            let username = parts[2].to_string();
            let password = parts[3].to_string();

            if host.is_empty() {
                return Err(raise_error!(
                    "Empty hostname in proxy URL.".into(),
                    ErrorCode::InvalidParameter
                ));
            }
            if username.is_empty() {
                return Err(raise_error!(
                    "Empty username in proxy URL.".into(),
                    ErrorCode::InvalidParameter
                ));
            }
            if password.is_empty() {
                return Err(raise_error!(
                    "Empty password in proxy URL.".into(),
                    ErrorCode::InvalidParameter
                ));
            }

            Ok(ProxyAddr {
                host,
                port,
                username: Some(username),
                password: Some(password),
            })
        }
        _ => Err(raise_error!(
            format!(
                "Invalid proxy URL format '{}'. Expected '[scheme://][user:pass@]host:port' or 'scheme://host:port:user:pass'.",
                input
            ),
            ErrorCode::InvalidParameter
        )),
    }
}

/// Split "user:pass" into (Some(user), Some(pass)), or "user" into (Some(user), None).
fn split_userinfo(userinfo: &str) -> BichonResult<(Option<String>, Option<String>)> {
    if userinfo.is_empty() {
        return Ok((None, None));
    }
    if let Some(colon_pos) = userinfo.find(':') {
        let user = &userinfo[..colon_pos];
        let pass = &userinfo[colon_pos + 1..];
        if user.is_empty() {
            return Err(raise_error!(
                "Empty username in proxy URL credentials.".into(),
                ErrorCode::InvalidParameter
            ));
        }
        Ok((Some(user.to_string()), Some(pass.to_string())))
    } else {
        Ok((Some(userinfo.to_string()), None))
    }
}

/// Split "host:port" into (host, port). Handles IPv6 addresses in brackets.
fn split_hostport(hostport: &str) -> BichonResult<(String, u16)> {
    if hostport.is_empty() {
        return Err(raise_error!(
            "Empty host:port in proxy URL.".into(),
            ErrorCode::InvalidParameter
        ));
    }

    // IPv6: [::1]:1080
    if hostport.starts_with('[') {
        let close_bracket = hostport.find(']').ok_or_else(|| {
            raise_error!(
                format!("Invalid IPv6 address in proxy URL: '{}'.", hostport),
                ErrorCode::InvalidParameter
            )
        })?;
        let host = hostport[1..close_bracket].to_string();
        let after_bracket = &hostport[close_bracket + 1..];
        if !after_bracket.starts_with(':') {
            return Err(raise_error!(
                format!("Missing port after IPv6 address in proxy URL: '{}'.", hostport),
                ErrorCode::InvalidParameter
            ));
        }
        let port = after_bracket[1..].parse::<u16>().map_err(|_| {
            raise_error!(
                format!("Invalid port in proxy URL: '{}'.", hostport),
                ErrorCode::InvalidParameter
            )
        })?;
        return Ok((host, port));
    }

    // hostname:port or ip:port — split from right
    let last_colon = hostport.rfind(':').ok_or_else(|| {
        raise_error!(
            format!("Missing port in proxy URL: '{}'.", hostport),
            ErrorCode::InvalidParameter
        )
    })?;
    let host = hostport[..last_colon].to_string();
    let port = hostport[last_colon + 1..].parse::<u16>().map_err(|_| {
        raise_error!(
            format!("Invalid port in proxy URL: '{}'.", hostport),
            ErrorCode::InvalidParameter
        )
    })?;

    if host.is_empty() {
        return Err(raise_error!(
            "Empty hostname in proxy URL.".into(),
            ErrorCode::InvalidParameter
        ));
    }

    Ok((host, port))
}

/// Try to connect via SOCKS5 proxy or TCP with timeout.
async fn connect_with_optional_proxy(
    use_proxy: Option<u64>,
    address: SocketAddr,
) -> BichonResult<TcpStream> {
    if let Some(proxy_id) = use_proxy {
        let proxy = Proxy::get(proxy_id)?;
        let addr = parse_proxy_url(&proxy.url)?;
        let proxy_addr = (addr.host.as_str(), addr.port);

        let result = if let (Some(ref user), Some(ref pass)) = (addr.username, addr.password) {
            timeout(
                TIMEOUT,
                Socks5Stream::connect_with_password(proxy_addr, address, user.as_str(), pass.as_str()),
            )
            .await
        } else {
            timeout(TIMEOUT, Socks5Stream::connect(proxy_addr, address)).await
        };

        return result
            .map_err(|_| {
                error!(
                    "SOCKS5 proxy connection to {} via {}:{} timed out after {}s",
                    address,
                    addr.host,
                    addr.port,
                    TIMEOUT.as_secs()
                );
                raise_error!(
                    format!(
                        "SOCKS5 proxy connection to {} via {}:{} timed out after {}s",
                        address,
                        addr.host,
                        addr.port,
                        TIMEOUT.as_secs()
                    ),
                    ErrorCode::ConnectionTimeout
                )
            })?
            .map(|s| s.into_inner())
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::NetworkError));
    }
    // Fallback to direct TCP connection
    timeout(TIMEOUT, TcpStream::connect(address))
        .await
        .map_err(|_| {
            error!(
                "TCP connection to {} timed out after {}s",
                address,
                TIMEOUT.as_secs()
            );
            raise_error!(
                format!(
                    "TCP connection to {} timed out after {}s",
                    address,
                    TIMEOUT.as_secs()
                ),
                ErrorCode::ConnectionTimeout
            )
        })?
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::NetworkError))
}
