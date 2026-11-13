import hashlib
import json
import os
import re
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from finalize_private_release import finalize
from private_release import check_new_version, check_versions, version_number


class PrivateReleaseTests(unittest.TestCase):
    def test_only_complete_verified_installers_are_promoted(self):
        root = Path(__file__).resolve().parents[2]
        version = re.search(r'^version = "([^"]+)"$', (root / 'Cargo.toml').read_text(), re.M)[1]
        tag = 'v' + version
        names = [f'rustdesk-{version}-{suffix}' for suffix in [
            'x86_64.exe', 'x86_64.msi', 'aarch64.dmg', 'x86_64.dmg',
            'x86_64.deb', '0.x86_64.rpm', '0.x86_64-suse.rpm']]
        for mode in ['complete', 'missing', 'tampered', 'wrong-commit', 'signing-failure']:
            with self.subTest(mode=mode):
                assets = [{'name': name, 'size': 2,
                           'digest': 'sha256:' + hashlib.sha256(b'ok').hexdigest()} for name in names]
                if mode == 'missing':
                    assets.pop()
                calls = []

                def fake_gh(*args):
                    calls.append(args)
                    if args[0] == 'api':
                        if '/git/ref/' in args[1]:
                            return json.dumps({'object': {'sha': 'other' if mode == 'wrong-commit' else 'commit'}})
                        return json.dumps({'prerelease': True, 'assets': assets})
                    if args[:2] == ('release', 'download'):
                        directory = Path(args[args.index('--dir') + 1])
                        for name in names:
                            (directory / name).write_bytes(b'no' if mode == 'tampered' else b'ok')
                    return ''

                with patch.dict(os.environ, VERSION=version, TAG_NAME=tag,
                                GITHUB_REPOSITORY='owner/repo', GITHUB_SHA='commit'), \
                        patch('finalize_private_release.gh', side_effect=fake_gh), \
                        patch('finalize_private_release.prepare_private_update', create=True,
                              side_effect=ValueError('signing failed') if mode == 'signing-failure' else None,
                              return_value=[]):
                    if mode == 'complete':
                        finalize()
                    else:
                        with self.assertRaises(ValueError):
                            finalize()
                self.assertEqual(any(call[:2] == ('release', 'edit') for call in calls), mode == 'complete')

    def test_private_manifest_is_signed_and_verified_for_all_seven_formats(self):
        import base64
        import shutil
        import subprocess
        from cryptography.hazmat.primitives import serialization
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
        from private_update_metadata import prepare_private_update
        project = Path(__file__).resolve().parents[2]
        key = Ed25519PrivateKey.generate()
        public = key.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        seed = key.private_bytes(serialization.Encoding.Raw, serialization.PrivateFormat.Raw,
                                 serialization.NoEncryption())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'src/update_metadata').mkdir(parents=True)
            shutil.copytree(project / 'res/update', root / 'res/update')
            (root / 'src/update_metadata/private.rs').write_text(
                'TrustedUpdateKey { key_id: "2026-ed25519-main", public_key: ['
                + ','.join(map(str, public)) + '] }')
            for suffix in ['x86_64.exe', 'x86_64.msi', 'aarch64.dmg', 'x86_64.dmg',
                           'x86_64.deb', '0.x86_64.rpm', '0.x86_64-suse.rpm']:
                (root / f'rustdesk-1.5.10-{suffix}').write_bytes(b'checked installer')
            with patch.dict(os.environ,
                            RUSTDESK_UPDATE_ED25519_SEED=base64.b64encode(seed).decode(),
                            RUSTDESK_UPDATE_ED25519_PUBLIC_KEY=base64.b64encode(public).decode()):
                paths = prepare_private_update(root, root, '1.5.10', 'v1.5.10',
                                               'akurtlua-lgtm/rustdesk-private-build')
                metadata = json.loads(paths[0].read_text())
                self.assertEqual(len(metadata['artifacts']), 7)
                self.assertEqual(len({(a['platform'], a['arch'], a['format'])
                                      for a in metadata['artifacts']}), 7)
                self.assertTrue(all(a['url'].startswith(
                    'https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/download/v1.5.10/')
                                    for a in metadata['artifacts']))
                os.environ['RUSTDESK_UPDATE_ED25519_PUBLIC_KEY'] = base64.b64encode(bytes(32)).decode()
                with self.assertRaises(subprocess.CalledProcessError):
                    prepare_private_update(root, root, '1.5.10', 'v1.5.10',
                                           'akurtlua-lgtm/rustdesk-private-build')

    def test_versions_and_build_number_must_match(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'flutter').mkdir()
            (root / 'res').mkdir()
            (root / 'libs/portable').mkdir(parents=True)
            (root / 'libs/portable/Cargo.toml').write_text('version = "1.5.4"\n')
            for name in ['rpm-flutter.spec', 'rpm-flutter-suse.spec']:
                (root / 'res' / name).write_text('Version:    1.5.4\n')
            (root / 'Cargo.toml').write_text('[package]\nversion = "1.5.4"\n')
            (root / 'Cargo.lock').write_text('[[package]]\nname = "rustdesk"\nversion = "1.5.4"\n[[package]]\nname = "rustdesk-portable-packer"\nversion = "1.5.4"\n')
            (root / 'flutter/pubspec.yaml').write_text('version: 1.5.4+72\n')
            check_versions(root, '1.5.4', 'v1.5.4')
            (root / 'flutter/pubspec.yaml').write_text('version: 1.5.3+71\n')
            with self.assertRaises(ValueError):
                check_versions(root, '1.5.4', 'v1.5.4')
            (root / 'flutter/pubspec.yaml').write_text('version: 1.5.4+72\n')
            (root / 'res/rpm-flutter.spec').write_text('Version:    1.5.3\n')
            with self.assertRaises(ValueError):
                check_versions(root, '1.5.4', 'v1.5.4')
            (root / 'res/rpm-flutter.spec').write_text('Version:    1.5.4\n')
            (root / 'libs/portable/Cargo.toml').write_text('version = "1.5.3"\n')
            with self.assertRaises(ValueError):
                check_versions(root, '1.5.4', 'v1.5.4')

    def test_republish_downgrade_and_nonstable_versions_are_refused(self):
        tags = 'abc refs/tags/private-1.5.3-account-fix\ndef refs/tags/v1.5.9\n'
        check_new_version('1.5.10', tags)
        for value in ['1.5.9', '1.5.4']:
            with self.assertRaises(ValueError):
                check_new_version(value, tags)
        for value in ['1.05.4', '1.5.4-beta', '1.5.4\n', '1.5.65536']:
            with self.assertRaises(ValueError):
                version_number(value)


if __name__ == '__main__':
    unittest.main()
