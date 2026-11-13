use super::{
    verify_update_metadata_with_options, ArtifactUrlPolicy, MetadataRequirements, TrustedUpdateKey,
    UpdateArtifactQuery, VerificationOptions, VerifiedUpdateArtifact,
};
use hbb_common::ResultType;

const PRIVATE_UPDATE_KEYS: &[TrustedUpdateKey] = &[TrustedUpdateKey {
    key_id: "2026-ed25519-main",
    algorithm: "ed25519",
    public_key: [
        151, 173, 251, 139, 100, 12, 239, 79, 4, 193, 19, 240, 133, 225, 133, 199, 108, 119, 157,
        62, 10, 121, 142, 14, 103, 84, 97, 122, 195, 163, 109, 201,
    ],
}];

pub(crate) fn verify_private_update_metadata(
    metadata: &[u8],
    signature: &[u8],
    version: &str,
    artifact: UpdateArtifactQuery<'_>,
) -> ResultType<VerifiedUpdateArtifact> {
    let release_id = format!("v{version}");
    let prefix = format!(
        "https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/download/{release_id}/"
    );
    verify_update_metadata_with_options(
        metadata,
        signature,
        VerificationOptions {
            requirements: MetadataRequirements {
                expected_version: version,
                expected_release_id: &release_id,
                artifact,
            },
            expected_package_id: "rustdesk-private-build",
            artifact_url_policy: ArtifactUrlPolicy::ExactPrefix(&prefix),
            trusted_keys: PRIVATE_UPDATE_KEYS,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_metadata_selects_our_signed_native_artifact() {
        let query = UpdateArtifactQuery {
            platform: "macos",
            arch: "aarch64",
            format: "dmg",
        };
        let artifact = verify_private_update_metadata(
            include_bytes!("fixtures/private-release.json"),
            include_bytes!("fixtures/private-release.json.sig"),
            "1.5.8",
            query,
        )
        .expect("our independently signed release");
        assert_eq!(artifact.url, "https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/download/v1.5.8/rustdesk-1.5.8-aarch64.dmg");
        assert_eq!(artifact.size, 30939236);
    }

    #[test]
    fn private_metadata_refuses_tampering_wrong_release_arch_and_signed_foreign_urls() {
        let metadata = include_bytes!("fixtures/private-release.json");
        let signature = include_bytes!("fixtures/private-release.json.sig");
        let query = UpdateArtifactQuery {
            platform: "macos",
            arch: "aarch64",
            format: "dmg",
        };
        let mut tampered = metadata.to_vec();
        tampered.push(b' ');
        assert!(verify_private_update_metadata(&tampered, signature, "1.5.8", query).is_err());
        assert!(verify_private_update_metadata(metadata, signature, "1.5.9", query).is_err());
        assert!(verify_private_update_metadata(
            metadata,
            signature,
            "1.5.8",
            UpdateArtifactQuery {
                arch: "x86_64",
                ..query
            }
        )
        .is_err());
        assert!(verify_private_update_metadata(
            include_bytes!("fixtures/private-foreign-release.json"),
            include_bytes!("fixtures/private-foreign-release.json.sig"),
            "1.5.8",
            query,
        )
        .is_err());
    }
}
