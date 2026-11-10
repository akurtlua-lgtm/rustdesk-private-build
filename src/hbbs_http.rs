use hbb_common::ResultType;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

#[cfg(feature = "flutter")]
pub mod account;
pub mod downloader;
mod http_client;
pub mod record_upload;
pub mod sync;
pub use http_client::{
    create_http_client_async, create_http_client_async_with_url_strict,
    create_http_client_with_url, create_http_client_with_url_strict, get_url_for_tls,
};

pub(crate) fn signalling_token(token: &str) -> &str {
    // OSS console sessions authenticate only the HTTPS API, not hbbs/hbbr.
    if token.starts_with("rdoss_v1_") {
        ""
    } else {
        token
    }
}

#[derive(Debug)]
pub enum HbbHttpResponse<T> {
    ErrorFormat,
    Error(String),
    DataTypeFormat,
    Data(T),
}

impl<T: DeserializeOwned> HbbHttpResponse<T> {
    pub fn parse(body: &str) -> ResultType<Self> {
        let map = serde_json::from_str::<Map<String, Value>>(body)?;
        if let Some(error) = map.get("error") {
            if let Some(err) = error.as_str() {
                Ok(Self::Error(err.to_owned()))
            } else {
                Ok(Self::ErrorFormat)
            }
        } else {
            match serde_json::from_value(Value::Object(map)) {
                Ok(v) => Ok(Self::Data(v)),
                Err(_) => Ok(Self::DataTypeFormat),
            }
        }
    }
}

#[cfg(test)]
mod signalling_token_tests {
    use super::signalling_token;

    #[test]
    fn console_session_is_not_sent_to_rendezvous_or_relay() {
        assert_eq!(signalling_token("rdoss_v1_console-session"), "");
    }

    #[test]
    fn existing_server_tokens_are_preserved() {
        for token in ["", "pro-session-token", "rdoss_v2_unknown"] {
            assert_eq!(signalling_token(token), token);
        }
    }
}
