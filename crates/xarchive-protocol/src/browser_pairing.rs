//! Bootstrap and authentication control messages for the browser bridge.
//!
//! These messages are deliberately separate from `BrowserRequest` and
//! `BrowserResponse`: bootstrap never reaches the archive business adapter.

use serde::{Deserialize, Serialize};

use crate::{BROWSER_PAIRING_PROTOCOL_VERSION, ProtocolError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum BrowserPairingRequest {
    Bootstrap {
        protocol_version: u32,
        request_id: String,
    },
}

/// Decode only pairing control messages. Business requests intentionally fail
/// this decoder and must continue through the existing BrowserRequest path.
pub fn decode_browser_pairing_request(
    payload: &str,
) -> Result<BrowserPairingRequest, serde_json::Error> {
    serde_json::from_str(payload)
}

impl BrowserPairingRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        let (version, request_id) = match self {
            Self::Bootstrap {
                protocol_version,
                request_id,
            } => (protocol_version, request_id),
        };
        validate_envelope(*version, request_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum BrowserPairingResponse {
    Bootstrap {
        protocol_version: u32,
        request_id: String,
        host: String,
        port: u16,
        path: String,
        runtime_instance_id: String,
        ticket: String,
        expires_in_ms: u32,
    },
    Error {
        protocol_version: u32,
        request_id: Option<String>,
        error_code: String,
        error_message: String,
        retryable: bool,
    },
}

impl BrowserPairingResponse {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Bootstrap {
                protocol_version,
                request_id,
                host,
                port,
                path,
                runtime_instance_id,
                ticket,
                expires_in_ms,
            } => {
                validate_envelope(*protocol_version, request_id)?;
                if host != "127.0.0.1" || *port == 0 || path != "/" {
                    return Err(ProtocolError::InvalidBrowserPairingEndpoint);
                }
                if runtime_instance_id.is_empty()
                    || runtime_instance_id.len() > 128
                    || !runtime_instance_id.is_ascii()
                    || ticket.len() != 64
                    || !ticket
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                    || *expires_in_ms == 0
                    || *expires_in_ms > 30_000
                {
                    return Err(ProtocolError::InvalidBrowserPairingTicket);
                }
                Ok(())
            }
            Self::Error {
                protocol_version,
                request_id,
                error_code,
                error_message,
                ..
            } => {
                if *protocol_version != BROWSER_PAIRING_PROTOCOL_VERSION {
                    return Err(ProtocolError::UnsupportedBrowserPairingVersion(
                        *protocol_version,
                    ));
                }
                if request_id.as_ref().is_some_and(|id| !valid_request_id(id))
                    || error_code.is_empty()
                    || error_code.len() > 64
                    || !error_code.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
                    || error_message.is_empty()
                    || error_message.len() > 512
                    || error_message.contains(|character: char| character.is_control())
                {
                    return Err(ProtocolError::InvalidBrowserPairingEnvelope);
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserPairingAuthentication {
    pub protocol_version: u32,
    pub message_type: String,
    pub ticket: String,
}

impl BrowserPairingAuthentication {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != BROWSER_PAIRING_PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedBrowserPairingVersion(
                self.protocol_version,
            ));
        }
        if self.message_type != "authenticate"
            || self.ticket.len() != 64
            || !self
                .ticket
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(ProtocolError::InvalidBrowserPairingTicket);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserPairingAuthenticationResponse {
    pub protocol_version: u32,
    pub message_type: String,
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

impl BrowserPairingAuthenticationResponse {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != BROWSER_PAIRING_PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedBrowserPairingVersion(
                self.protocol_version,
            ));
        }
        if self.message_type != "authentication_response"
            || (self.authenticated && self.error_code.is_some())
            || (!self.authenticated && self.error_code.as_deref() != Some("AUTHENTICATION_FAILED"))
        {
            return Err(ProtocolError::InvalidBrowserPairingEnvelope);
        }
        Ok(())
    }
}

fn validate_envelope(version: u32, request_id: &str) -> Result<(), ProtocolError> {
    if version != BROWSER_PAIRING_PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedBrowserPairingVersion(version));
    }
    if !valid_request_id(request_id) {
        return Err(ProtocolError::InvalidBrowserPairingEnvelope);
    }
    Ok(())
}

fn valid_request_id(request_id: &str) -> bool {
    (1..=128).contains(&request_id.len())
        && request_id.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_request_round_trips_and_rejects_unknown_fields() {
        let request = BrowserPairingRequest::Bootstrap {
            protocol_version: BROWSER_PAIRING_PROTOCOL_VERSION,
            request_id: "bootstrap-1".into(),
        };
        let encoded = serde_json::to_string(&request).expect("serialize");
        assert_eq!(
            serde_json::from_str::<BrowserPairingRequest>(&encoded).expect("deserialize"),
            request
        );
        assert!(serde_json::from_str::<BrowserPairingRequest>(
            r#"{"message_type":"bootstrap","protocol_version":1,"request_id":"r1","token":"legacy"}"#
        )
        .is_err());
        let non_ascii_id = BrowserPairingRequest::Bootstrap {
            protocol_version: 1,
            request_id: "配对".into(),
        };
        assert_eq!(
            non_ascii_id.validate(),
            Err(ProtocolError::InvalidBrowserPairingEnvelope)
        );
        let control_id = BrowserPairingRequest::Bootstrap {
            protocol_version: 1,
            request_id: "line\nbreak".into(),
        };
        assert_eq!(
            control_id.validate(),
            Err(ProtocolError::InvalidBrowserPairingEnvelope)
        );
    }

    #[test]
    fn bootstrap_response_requires_loopback_endpoint_and_bounded_ticket_fields() {
        let response = BrowserPairingResponse::Bootstrap {
            protocol_version: BROWSER_PAIRING_PROTOCOL_VERSION,
            request_id: "bootstrap-1".into(),
            host: "127.0.0.1".into(),
            port: 43127,
            path: "/".into(),
            runtime_instance_id: "runtime-1".into(),
            ticket: "a".repeat(64),
            expires_in_ms: 30_000,
        };
        assert_eq!(response.validate(), Ok(()));
        let encoded = serde_json::to_string(&response).expect("serialize");
        assert_eq!(
            serde_json::from_str::<BrowserPairingResponse>(&encoded).expect("deserialize"),
            response
        );

        let remote = BrowserPairingResponse::Bootstrap {
            protocol_version: 1,
            request_id: "r1".into(),
            host: "192.0.2.1".into(),
            port: 43127,
            path: "/".into(),
            runtime_instance_id: "runtime-1".into(),
            ticket: "a".repeat(64),
            expires_in_ms: 30_000,
        };
        assert_eq!(
            remote.validate(),
            Err(ProtocolError::InvalidBrowserPairingEndpoint)
        );

        let oversized_ttl = BrowserPairingResponse::Bootstrap {
            protocol_version: 1,
            request_id: "r1".into(),
            host: "127.0.0.1".into(),
            port: 43127,
            path: "/".into(),
            runtime_instance_id: "runtime-1".into(),
            ticket: "a".repeat(64),
            expires_in_ms: 30_001,
        };
        assert_eq!(
            oversized_ttl.validate(),
            Err(ProtocolError::InvalidBrowserPairingTicket)
        );
    }

    #[test]
    fn authentication_messages_validate_version_and_semantics() {
        let auth = BrowserPairingAuthentication {
            protocol_version: 1,
            message_type: "authenticate".into(),
            ticket: "b".repeat(64),
        };
        assert_eq!(auth.validate(), Ok(()));
        let malformed_ticket = BrowserPairingAuthentication {
            ticket: "short".into(),
            ..auth.clone()
        };
        assert_eq!(
            malformed_ticket.validate(),
            Err(ProtocolError::InvalidBrowserPairingTicket)
        );
        let accepted = BrowserPairingAuthenticationResponse {
            protocol_version: 1,
            message_type: "authentication_response".into(),
            authenticated: true,
            error_code: None,
        };
        assert_eq!(accepted.validate(), Ok(()));
        let rejected = BrowserPairingAuthenticationResponse {
            authenticated: false,
            error_code: Some("AUTHENTICATION_FAILED".into()),
            ..accepted
        };
        assert_eq!(rejected.validate(), Ok(()));

        let control_character_error = BrowserPairingResponse::Error {
            protocol_version: 1,
            request_id: Some("r1".into()),
            error_code: "BAD\nCODE".into(),
            error_message: "safe".into(),
            retryable: false,
        };
        assert_eq!(
            control_character_error.validate(),
            Err(ProtocolError::InvalidBrowserPairingEnvelope)
        );
    }

    #[test]
    fn pairing_request_rejects_unsupported_version_and_invalid_request_id() {
        let unsupported = BrowserPairingRequest::Bootstrap {
            protocol_version: 2,
            request_id: "r1".into(),
        };
        assert_eq!(
            unsupported.validate(),
            Err(ProtocolError::UnsupportedBrowserPairingVersion(2))
        );
        let invalid = BrowserPairingRequest::Bootstrap {
            protocol_version: 1,
            request_id: "".into(),
        };
        assert_eq!(
            invalid.validate(),
            Err(ProtocolError::InvalidBrowserPairingEnvelope)
        );
    }
}
