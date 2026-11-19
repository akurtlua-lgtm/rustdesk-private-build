use url::Url;

const DISABLED_API: &str = "https://configuration.invalid";

fn configured_api(api: Option<&str>) -> Option<&str> {
    api.map(|value| {
        let valid = value.trim() == value
            && Url::parse(value).is_ok_and(|url| {
                url.scheme() == "https"
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
            });
        if valid {
            value
        } else {
            DISABLED_API
        }
    })
}

#[cfg(any(target_os = "android", target_os = "ios", target_env = "ohos"))]
pub(crate) fn load_api_server() {
    use hbb_common::{config, log};

    let Some(api) = configured_api(option_env!("RD_PRIVATE_API_SERVER")) else {
        return;
    };
    if api == DISABLED_API {
        log::error!("Private mobile HTTPS API configuration is invalid");
    }
    config::DEFAULT_SETTINGS
        .write()
        .unwrap()
        .insert("api-server".to_owned(), api.to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_https_api_prefix() {
        assert_eq!(
            configured_api(Some("https://example.com/private")),
            Some("https://example.com/private")
        );
    }

    #[test]
    fn rejects_insecure_or_credential_bearing_api_without_http_fallback() {
        for api in [
            "http://example.com/private",
            "https://user:password@example.com/private",
            "https://example.com/private?token=test",
            "https://example.com/private#fragment",
            " https://example.com/private",
            "",
        ] {
            assert_eq!(configured_api(Some(api)), Some(DISABLED_API));
        }
    }

    #[test]
    fn unconfigured_build_retains_existing_defaults() {
        assert_eq!(configured_api(None), None);
    }
}
