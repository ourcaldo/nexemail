// check-if-email-exists
// Copyright (C) 2018-2023 Reacher

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::gmail::GmailError;
use super::headless::HeadlessError;
use super::outlook::microsoft365::Microsoft365Error;
use super::parser;
use super::yahoo::YahooError;
use crate::util::ser_with_display::ser_with_display;
use async_smtp::error::Error as AsyncSmtpError;
use serde::Serialize;
use std::time::Duration;
use thiserror::Error;

/// Error occurred connecting to this email server via SMTP.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", content = "message")]
pub enum SmtpError {
        /// Error when verifying a Yahoo email via HTTP requests.
        #[error("Yahoo error: {0}")]
        YahooError(YahooError),
        /// Error when verifying a Gmail email via a HTTP request.
        #[error("Gmail error: {0}")]
        GmailError(GmailError),
        /// Error when verifying a Hotmail email via headless browser.
        #[error("Headless verification error: {0}")]
        HeadlessError(HeadlessError),
        /// Error when verifying a Microsoft 365 email via HTTP request.
        #[error("Microsoft 365 API error: {0}")]
        Microsoft365Error(Microsoft365Error),
        /// Error from async-smtp crate.
        #[error("SMTP error: {0}")]
        #[serde(serialize_with = "ser_with_display")]
        AsyncSmtpError(AsyncSmtpError),
        /// I/O error.
        #[error("I/O error: {0}")]
        #[serde(serialize_with = "ser_with_display")]
        IOError(std::io::Error),
        /// Timeout error.
        #[error("Timeout error: {0:?}")]
        Timeout(Duration),
        /// SOCKS5 proxy error.
        #[error("SOCKS5 error: {0}")]
        #[serde(serialize_with = "ser_with_display")]
        Socks5(fast_socks5::SocksError),
        /// Anyhow error.
        /// This is a catch-all error type for any error that can't be categorized
        /// into the above types.
        #[error("Anyhow error: {0}")]
        #[serde(serialize_with = "ser_with_display")]
        AnyhowError(anyhow::Error),
}

impl From<YahooError> for SmtpError {
        fn from(e: YahooError) -> Self {
                SmtpError::YahooError(e)
        }
}

impl From<GmailError> for SmtpError {
        fn from(e: GmailError) -> Self {
                SmtpError::GmailError(e)
        }
}

impl From<HeadlessError> for SmtpError {
        fn from(e: HeadlessError) -> Self {
                SmtpError::HeadlessError(e)
        }
}

impl From<Microsoft365Error> for SmtpError {
        fn from(e: Microsoft365Error) -> Self {
                SmtpError::Microsoft365Error(e)
        }
}

impl From<AsyncSmtpError> for SmtpError {
        fn from(e: AsyncSmtpError) -> Self {
                SmtpError::AsyncSmtpError(e)
        }
}

impl From<std::io::Error> for SmtpError {
        fn from(e: std::io::Error) -> Self {
                SmtpError::IOError(e)
        }
}

impl From<fast_socks5::SocksError> for SmtpError {
        fn from(e: fast_socks5::SocksError) -> Self {
                SmtpError::Socks5(e)
        }
}

impl From<anyhow::Error> for SmtpError {
        fn from(e: anyhow::Error) -> Self {
                SmtpError::AnyhowError(e)
        }
}

impl SmtpError {
        /// Get a human-understandable description of the error, in form of an enum
        /// SmtpErrorDesc. This only parses the following known errors:
        /// - IP blacklisted
        /// - IP needs reverse DNS
        pub fn get_description(&self) -> Option<SmtpErrorDesc> {
                match self {
                        SmtpError::AsyncSmtpError(_) => {
                                if parser::is_err_ip_blacklisted(self) {
                                        Some(SmtpErrorDesc::IpBlacklisted)
                                } else if parser::is_err_needs_rdns(self) {
                                        Some(SmtpErrorDesc::NeedsRDNS)
                                } else {
                                        None
                                }
                        }
                        _ => None,
                }
        }

        /// Get a detailed, human-readable description of a SOCKS5 error.
        /// This provides much more specific information than the default Display implementation.
        pub fn get_detailed_socks5_description(&self) -> Option<String> {
                match self {
                        SmtpError::Socks5(socks_error) => Some(format_socks5_error_detailed(socks_error)),
                        _ => None,
                }
        }
}

/// Format a SOCKS5 error with detailed, specific information about what went wrong.
/// This replaces generic messages like "General failure" with actionable descriptions.
pub fn format_socks5_error_detailed(error: &fast_socks5::SocksError) -> String {
        use fast_socks5::SocksError;

        match error {
                SocksError::Io(io_err) => {
                        let kind = io_err.kind();
                        let details = match kind {
                                std::io::ErrorKind::ConnectionRefused => {
                                        "Connection refused - the SOCKS5 proxy server is not accepting connections. \
                                        Verify the proxy is running and the port is correct."
                                }
                                std::io::ErrorKind::ConnectionReset => {
                                        "Connection reset by proxy - the SOCKS5 server terminated the connection unexpectedly. \
                                        The proxy may be overloaded or blocking this connection."
                                }
                                std::io::ErrorKind::TimedOut => {
                                        "Connection timed out - unable to reach the SOCKS5 proxy server within the timeout period. \
                                        Check network connectivity and firewall rules."
                                }
                                std::io::ErrorKind::ConnectionAborted => {
                                        "Connection aborted - the connection to the SOCKS5 proxy was terminated. \
                                        This may indicate network instability or proxy issues."
                                }
                                std::io::ErrorKind::NotConnected => {
                                        "Not connected - failed to establish connection to the SOCKS5 proxy. \
                                        Verify proxy host and port are correct."
                                }
                                std::io::ErrorKind::AddrNotAvailable => {
                                        "Address not available - the SOCKS5 proxy address could not be resolved or is invalid."
                                }
                                std::io::ErrorKind::AddrInUse => {
                                        "Address in use - local address conflict when connecting to SOCKS5 proxy."
                                }
                                std::io::ErrorKind::PermissionDenied => {
                                        "Permission denied - insufficient permissions to connect to the SOCKS5 proxy. \
                                        This may be a firewall or system policy issue."
                                }
                                std::io::ErrorKind::UnexpectedEof => {
                                        "Unexpected end of stream - the SOCKS5 proxy closed the connection prematurely. \
                                        The proxy may have crashed or rejected the request."
                                }
                                _ => "I/O error occurred while communicating with the SOCKS5 proxy.",
                        };
                        format!(
                                "SOCKS5 I/O error ({}): {} Raw error: {}",
                                kind, details, io_err
                        )
                }
                SocksError::ReplyError(reply_error) => {
                        format_socks5_reply_error(reply_error)
                }
                SocksError::AuthenticationFailed(msg) => {
                        format!(
                                "SOCKS5 authentication failed: {}. \
                                Verify your proxy username and password are correct.",
                                msg
                        )
                }
                SocksError::AuthenticationRejected(msg) => {
                        format!(
                                "SOCKS5 authentication rejected by proxy: {}. \
                                The proxy server does not accept the provided credentials or authentication method.",
                                msg
                        )
                }
                SocksError::AuthMethodUnacceptable(methods) => {
                        format!(
                                "SOCKS5 authentication method not accepted. Requested methods: {:?}. \
                                The proxy requires a different authentication method than what was offered.",
                                methods
                        )
                }
                SocksError::UnsupportedSocksVersion(version) => {
                        format!(
                                "Unsupported SOCKS version: {}. Expected SOCKS5 (version 5). \
                                The server may not support SOCKS5 or is running a different protocol.",
                                version
                        )
                }
                SocksError::InvalidHeader { expected, found } => {
                        format!(
                                "Invalid SOCKS5 protocol header. Expected: {}, Found: {}. \
                                The proxy may not be a valid SOCKS5 server or there's a protocol mismatch.",
                                expected, found
                        )
                }
                SocksError::ExceededMaxDomainLen(len) => {
                        format!(
                                "Domain name too long for SOCKS5 protocol: {} bytes (max 255). \
                                The target domain name exceeds SOCKS5 protocol limits.",
                                len
                        )
                }
                SocksError::ArgumentInputError(msg) => {
                        format!(
                                "Invalid SOCKS5 connection argument: {}. \
                                Check the proxy configuration parameters.",
                                msg
                        )
                }
                SocksError::Redaction(msg) => {
                        format!("SOCKS5 error (redacted for security): {}", msg)
                }
                SocksError::Other(anyhow_err) => {
                        format!(
                                "SOCKS5 unexpected error: {}. \
                                This is an unclassified error from the proxy connection.",
                                anyhow_err
                        )
                }
        }
}

/// Format SOCKS5 reply error codes with detailed, actionable descriptions.
fn format_socks5_reply_error(reply_error: &fast_socks5::ReplyError) -> String {
        use fast_socks5::ReplyError;

        match reply_error {
                ReplyError::Succeeded => {
                        "SOCKS5 connection succeeded (this should not appear as an error).".to_string()
                }
                ReplyError::GeneralFailure => {
                        "SOCKS5 General Failure (reply code 0x01): The proxy server encountered an internal error \
                        and could not complete the request. Possible causes: \
                        (1) The proxy cannot reach the target SMTP server - check if the target is accessible from the proxy's network. \
                        (2) The proxy has internal configuration issues or is overloaded. \
                        (3) Firewall or security policy on the proxy is blocking this connection. \
                        (4) The proxy's outbound network is restricted. \
                        Try using a different proxy or verify the target server is reachable from the proxy's location.".to_string()
                }
                ReplyError::ConnectionNotAllowed => {
                        "SOCKS5 Connection Not Allowed (reply code 0x02): The proxy's ruleset explicitly denies this connection. \
                        The proxy administrator has configured policies that block connections to this target. \
                        This may be due to: (1) IP-based access control lists, (2) Domain blocking rules, \
                        (3) Port restrictions (SMTP port 25 is often blocked), (4) Rate limiting or abuse prevention. \
                        Contact the proxy provider or use a different proxy.".to_string()
                }
                ReplyError::NetworkUnreachable => {
                        "SOCKS5 Network Unreachable (reply code 0x03): The proxy cannot route traffic to the target network. \
                        The target SMTP server's network is not accessible from the proxy. \
                        Possible causes: (1) No route exists to the target network, (2) Network partition or outage, \
                        (3) The proxy's network configuration doesn't include this route. \
                        Try a proxy in a different geographic location.".to_string()
                }
                ReplyError::HostUnreachable => {
                        "SOCKS5 Host Unreachable (reply code 0x04): The proxy could not reach the target SMTP server host. \
                        The specific host is not responding or is unreachable. \
                        Possible causes: (1) The SMTP server is down or offline, (2) DNS resolution failed on the proxy side, \
                        (3) The host is blocking connections from the proxy's IP, (4) Firewall blocking at the destination. \
                        Verify the target email domain's MX servers are operational.".to_string()
                }
                ReplyError::ConnectionRefused => {
                        "SOCKS5 Connection Refused (reply code 0x05): The target SMTP server actively refused the connection. \
                        The SMTP server is reachable but declined to accept the connection. \
                        Possible causes: (1) The SMTP server is not accepting connections on this port, \
                        (2) The proxy's IP address is blacklisted by the SMTP server, \
                        (3) Rate limiting or connection limits on the target server, \
                        (4) The SMTP service is temporarily unavailable. \
                        Try a different proxy with a clean IP reputation.".to_string()
                }
                ReplyError::TtlExpired => {
                        "SOCKS5 TTL Expired (reply code 0x06): The connection attempt timed out due to TTL expiration. \
                        The network path to the target is too long or congested. \
                        This typically indicates severe network latency or routing problems between the proxy and target.".to_string()
                }
                ReplyError::CommandNotSupported => {
                        "SOCKS5 Command Not Supported (reply code 0x07): The proxy does not support the CONNECT command. \
                        This is unusual for a SOCKS5 proxy as CONNECT is a basic command. \
                        The proxy may have limited functionality or be misconfigured.".to_string()
                }
                ReplyError::AddressTypeNotSupported => {
                        "SOCKS5 Address Type Not Supported (reply code 0x08): The proxy does not support the address format used. \
                        The target address type (IPv4/IPv6/domain) is not supported by this proxy. \
                        Try using a different address format or a proxy with broader address support.".to_string()
                }
                ReplyError::ConnectionTimeout => {
                        "SOCKS5 Connection Timeout: The connection to the proxy or target server timed out. \
                        The proxy server or target SMTP server did not respond in time. \
                        Possible causes: (1) Network latency is too high, (2) The proxy or target is overloaded, \
                        (3) Connection was blocked by a firewall without sending a rejection. \
                        Try increasing timeout values or using a proxy with lower latency.".to_string()
                }
        }
}

#[derive(Debug, Serialize)]
/// SmtpErrorDesc describes a description of which category the error belongs
/// to.
pub enum SmtpErrorDesc {
        /// The IP is blacklisted.
        IpBlacklisted,
        /// The IP needs a reverse DNS entry.
        NeedsRDNS,
}
