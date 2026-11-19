use url::Url;

fn endpoint(base: &str, operation: &str) -> Option<(Url, bool)> {
    let path = match operation {
        "register" => "/api/register",
        "login" => "/api/login",
        "me" => "/api/currentUser",
        "peers" => "/api/peers?current=1&pageSize=100",
        "devices" | "registerDevice" | "updateDevice" => "/api/devices",
        "logout" => "/api/logout",
        _ => return None,
    };
    let base = Url::parse(base).ok()?;
    if base.scheme() != "https"
        || base.host_str().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return None;
    }
    let url = Url::parse(&format!("{}{path}", base.as_str().trim_end_matches('/'))).ok()?;
    Some((url, matches!(operation, "peers" | "devices")))
}

#[cfg(target_env = "ohos")]
pub(crate) fn request(operation: &str, payload: &str, token: &str) -> serde_json::Value {
    use hbb_common::config::Config;
    use serde_json::{json, Value};
    use std::{io::Read, time::Duration};

    let base = crate::get_api_server(
        Config::get_option("api-server"),
        Config::get_option("custom-rendezvous-server"),
    );
    let Some((mut url, get)) = endpoint(&base, operation) else {
        return json!({"status":0,"error":"Invalid HTTPS account configuration or operation"});
    };
    if payload.len() > 65536 {
        return json!({"status":0,"error":"Account request is too large"});
    }
    let Ok(body) = serde_json::from_str::<Value>(payload) else {
        return json!({"status":0,"error":"Invalid account request"});
    };
    if !body.is_object() {
        return json!({"status":0,"error":"Invalid account request"});
    }
    if operation == "updateDevice" {
        let Some(id) = body.get("did").and_then(Value::as_u64).filter(|id| *id > 0) else {
            return json!({"status":0,"error":"Invalid device record"});
        };
        url.set_path(&format!("{}/{id}", url.path()));
    }
    let authenticated = !matches!(operation, "register" | "login");
    if authenticated
        && (!token.starts_with("rdoss_v1_")
            || token.len() > 4096
            || !token.bytes().all(|byte| byte.is_ascii_graphic()))
    {
        return json!({"status":401,"error":"Account login required"});
    }
    // Called only by the NAPI async worker, never from the UI or session Tokio runtime.
    let client = match reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(_) => return json!({"status":0,"error":"Unable to initialize secure account client"}),
    };
    let mut request = if get {
        client.get(url)
    } else if operation == "updateDevice" {
        client.patch(url).json(&body)
    } else {
        client.post(url).json(&body)
    };
    if authenticated {
        request = request.bearer_auth(token);
    }
    let response = match request.send() {
        Ok(response) => response,
        Err(_) => return json!({"status":0,"error":"Secure account request failed"}),
    };
    let status = response.status().as_u16();
    if (300..400).contains(&status) {
        return json!({"status":0,"error":"Account redirects are refused"});
    }
    const LIMIT: u64 = 4 * 1024 * 1024;
    let mut bytes = Vec::new();
    if response.take(LIMIT + 1).read_to_end(&mut bytes).is_err() || bytes.len() as u64 > LIMIT {
        return json!({"status":0,"error":"Unable to read bounded account response"});
    }
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(body) => json!({"status":status,"body":body}),
        Err(_) => json!({"status":0,"error":"Invalid account response"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_private_prefix_and_fixed_operations() {
        let (url, get) = endpoint("https://example.com/private/", "peers").unwrap();
        assert_eq!(
            url.as_str(),
            "https://example.com/private/api/peers?current=1&pageSize=100"
        );
        assert!(get);
        assert!(endpoint("https://example.com/private", "https://other.example").is_none());
    }

    #[test]
    fn refuses_insecure_or_credential_bearing_base() {
        for base in [
            "http://example.com",
            "https://user:pass@example.com",
            "https://example.com?token=x",
            "https://example.com#fragment",
        ] {
            assert!(endpoint(base, "login").is_none());
        }
    }
}
