use std::fmt;
use thiserror::Error;

use crate::redaction::redacted_identifier;

#[derive(Clone, Eq, PartialEq)]
pub struct RequestContext {
    pub request_id: String,
    pub operation: &'static str,
    account: String,
}

impl RequestContext {
    pub(crate) fn new(
        request_id: impl Into<String>,
        operation: &'static str,
        account_alias: &str,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            operation,
            account: redacted_identifier("account", account_alias),
        }
    }
}

impl fmt::Debug for RequestContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RequestContext")
            .field("request_id", &self.request_id)
            .field("operation", &self.operation)
            .field("account", &self.account)
            .finish()
    }
}

impl fmt::Display for RequestContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "operation={} request_id={} account={}",
            self.operation, self.request_id, self.account
        )
    }
}

#[derive(Debug, Error)]
pub enum WeixinApiError {
    #[error("{context}: request timed out")]
    Timeout { context: RequestContext },
    #[error("{context}: network request failed category={category}")]
    Network {
        context: RequestContext,
        category: &'static str,
    },
    #[error("{context}: HTTP request failed status={status}")]
    HttpStatus {
        context: RequestContext,
        status: u16,
    },
    #[error("{context}: response exceeds limit limit_bytes={limit_bytes}")]
    ResponseTooLarge {
        context: RequestContext,
        limit_bytes: usize,
    },
    #[error("{context}: API rejected request code={code}")]
    Api { context: RequestContext, code: i64 },
    #[error("{context}: response was not valid JSON")]
    InvalidJson { context: RequestContext },
    #[error("{context}: protocol response is missing or has an invalid critical field")]
    Protocol { context: RequestContext },
}
