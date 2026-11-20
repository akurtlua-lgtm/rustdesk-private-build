import hashlib
import json
import os
import sys
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from mobile_preview_release import apk_payload, check_record, execute, main


class MobileReleaseTests(unittest.TestCase):
    def test_source_and_payload_must_match_the_verified_build(self):
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / 'app.apk'
            artifact.write_bytes(b'compiled APK')
            record = dict(version='1.5.14', build_number='82', application_id='private.app',
                          source='a' * 40, run_id='123', artifact=artifact.name,
                          size=artifact.stat().st_size, sha256=hashlib.sha256(artifact.read_bytes()).hexdigest())
            check_record(record, artifact, '1.5.14', '82', 'private.app', 'a' * 40, '123')
            with self.assertRaisesRegex(ValueError, 'source/identity'):
                check_record(record, artifact, '1.5.14', '82', 'private.app', 'b' * 40, '123')
            artifact.write_bytes(b'changed APK!')
            with self.assertRaisesRegex(ValueError, 'digest'):
                check_record(record, artifact, '1.5.14', '82', 'private.app', 'a' * 40, '123')

    def test_long_term_signing_secrets_do_not_reach_preflight_children(self):
        environment = dict(GITHUB_REPOSITORY='akurtlua-lgtm/rustdesk-private-build',
                           GITHUB_REF='refs/heads/main', GITHUB_EVENT_NAME='workflow_dispatch',
                           GITHUB_SHA='a' * 40, GITHUB_RUN_ID='123', VERSION='1.5.14',
                           BUILD_NUMBER='82', MOBILE_APPLICATION_ID='private.app',
                           RD_PRIVATE_ANDROID_KEYSTORE_BASE64='test-only',
                           RD_PRIVATE_ANDROID_KEYSTORE_PASSWORD='test-only')
        def first_api_call(*_args, **_kwargs):
            output = execute(sys.executable, '-c',
                             "import os; print(any(k.startswith('RD_PRIVATE_ANDROID_KEYSTORE') for k in os.environ))")
            self.assertEqual(output.strip(), 'False')
            raise RuntimeError('preflight reached safely')
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, environment), \
                patch('mobile_preview_release.gh_api', side_effect=first_api_call):
            original = Path.cwd()
            try:
                os.chdir(directory)
                root = Path('mobile-preview-input')
                root.mkdir()
                for name, platform, artifact_name in [
                    ('android-build.json', 'android', 'rustdesk-1.5.14-android-arm64-unsigned.apk'),
                    ('ios-simulator-build.json', 'ios-simulator', 'rustdesk-1.5.14-ios-simulator-arm64.app.zip'),
                ]:
                    artifact = root / artifact_name
                    artifact.write_bytes(b'payload')
                    record = dict(version='1.5.14', build_number='82', application_id='private.app',
                                  source='a' * 40, run_id='123', artifact=artifact_name,
                                  platform=platform, size=7, sha256=hashlib.sha256(b'payload').hexdigest())
                    (root / name).write_text(json.dumps(record))
                with self.assertRaisesRegex(RuntimeError, 'preflight reached safely'):
                    main()
            finally:
                os.chdir(original)

    def test_signing_may_only_add_signature_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory) / name for name in ['unsigned.apk', 'signed.apk', 'tampered.apk']]
            for index, path in enumerate(paths):
                with zipfile.ZipFile(path, 'w') as archive:
                    archive.writestr('AndroidManifest.xml', b'unchanged' if index < 2 else b'changed')
                    if index > 0:
                        archive.writestr('META-INF/CERT.RSA', b'certificate')
            self.assertEqual(apk_payload(paths[0]), apk_payload(paths[1]))
            self.assertNotEqual(apk_payload(paths[0]), apk_payload(paths[2]))


if __name__ == '__main__':
    unittest.main()
