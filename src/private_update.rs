use hbb_common::{bail, ResultType};
use serde_derive::Deserialize;
use std::{io::Read, time::Duration};

const API_URL: &str =
    "https://api.github.com/repos/akurtlua-lgtm/rustdesk-private-build/releases/latest";
const RELEASE_PREFIX: &str =
    "https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/tag/";

pub(crate) fn enabled() -> bool {
    option_env!("RD_ID_SERVER").is_some_and(|value| !value.is_empty())
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
}

fn version(value: &str) -> Option<[u16; 3]> {
    let (major, rest) = value.split_once('.')?;
    let (minor, patch) = rest.split_once('.')?;
    let parse = |part: &str| {
        if part.is_empty()
            || (part.len() > 1 && part.starts_with('0'))
            || !part.bytes().all(|c| c.is_ascii_digit())
        {
            None
        } else {
            part.parse::<u16>().ok()
        }
    };
    Some([parse(major)?, parse(minor)?, parse(patch)?])
}

pub(crate) fn release_version(url: &str) -> Option<&str> {
    let value = url.strip_prefix(RELEASE_PREFIX)?.strip_prefix('v')?;
    version(value)?;
    Some(value)
}

fn newer_release(release: Release, current: &str) -> Option<String> {
    if release.draft
        || release.prerelease
        || release.html_url != format!("{RELEASE_PREFIX}{}", release.tag_name)
    {
        return None;
    }
    let latest = release_version(&release.html_url)?;
    (version(latest)? > version(current)?).then_some(release.html_url)
}

pub(crate) fn check() -> ResultType<()> {
    if !enabled() {
        return Ok(());
    }
    let response = crate::hbbs_http::create_http_client_with_url_strict(API_URL)?
        .get(API_URL)
        .header(
            "User-Agent",
            concat!("RustDesk-private/", env!("CARGO_PKG_VERSION")),
        )
        .timeout(Duration::from_secs(15))
        .send()?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(());
    }
    let response = response.error_for_status()?;
    let mut bytes = Vec::new();
    response.take(131_073).read_to_end(&mut bytes)?;
    if bytes.len() > 131_072 {
        bail!("Private release response is too large");
    }
    if let Some(url) = newer_release(serde_json::from_slice(&bytes)?, crate::VERSION) {
        *crate::common::SOFTWARE_UPDATE_URL.lock().unwrap() = url.clone();
        #[cfg(feature = "flutter")]
        let _ = crate::flutter::push_global_event(
            crate::flutter::APP_TYPE_MAIN,
            serde_json::json!({"name": "check_software_update_finish", "url": url}).to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.to_owned(),
            html_url: format!("{RELEASE_PREFIX}{tag}"),
            draft: false,
            prerelease: false,
        }
    }

    #[test]
    fn only_newer_complete_stable_versions_are_offered() {
        assert!(newer_release(release("v1.5.10"), "1.5.4").is_some());
        for tag in ["v1.5.4", "v1.5.3", "v1.05.5", "v1.5.5-beta", "v1.5.5/extra"] {
            assert!(newer_release(release(tag), "1.5.4").is_none(), "{tag}");
        }
        let mut draft = release("v1.5.5");
        draft.draft = true;
        assert!(newer_release(draft, "1.5.4").is_none());
        let mut preview = release("v1.5.5");
        preview.prerelease = true;
        assert!(newer_release(preview, "1.5.4").is_none());
    }

    #[test]
    fn foreign_or_mismatched_release_links_are_refused() {
        for url in [
            "https://github.com/rustdesk/rustdesk/releases/tag/v1.5.5",
            "https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/tag/v1.5.5?next=evil",
            "http://github.com/akurtlua-lgtm/rustdesk-private-build/releases/tag/v1.5.5",
        ] {
            assert!(release_version(url).is_none());
        }
        let mut mismatch = release("v1.5.5");
        mismatch.html_url = format!("{RELEASE_PREFIX}v1.5.6");
        assert!(newer_release(mismatch, "1.5.4").is_none());
    }
}
