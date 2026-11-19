from pathlib import Path
import json
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from mobile_build_error import safe_diagnostics
from verify_mobile_artifacts import verify_android, verify_ohos


class MobileArtifactTests(unittest.TestCase):
    def test_apk_with_wrong_identity_or_intel_core_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            apk = Path(directory) / 'test.apk'
            with zipfile.ZipFile(apk, 'w') as archive:
                for name in ('librustdesk.so', 'libflutter.so', 'libapp.so', 'libc++_shared.so'):
                    data = bytearray(64)
                    data[:6] = b'\x7fELF\x02\x01'
                    data[18:20] = (62 if name == 'librustdesk.so' else 183).to_bytes(2, 'little')
                    archive.writestr('lib/arm64-v8a/' + name, data)
            with patch('verify_mobile_artifacts.run', return_value="package: name='private.app' versionCode='82' versionName='1.5.14'"):
                with self.assertRaisesRegex(ValueError, 'identity/version'):
                    verify_android(apk, '1.5.14', '82', 'other.app', 'aapt2')
                with self.assertRaisesRegex(ValueError, 'ARM64 ELF'):
                    verify_android(apk, '1.5.14', '82', 'private.app', 'aapt2')

    def test_hap_refuses_wrong_identity_and_embedded_build_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            hap = Path(directory) / 'test.hap'
            info = {'app': {'bundleName': 'private.app', 'versionName': '1.5.14',
                            'versionCode': 82, 'minAPIVersion': 17,
                            'targetAPIVersion': 20, 'compileSdkType': 'OpenHarmony'},
                    'module': {'requestPermissions': [{'name': 'ohos.permission.INTERNET'}]}}
            with zipfile.ZipFile(hap, 'w') as archive:
                archive.writestr('module.json', json.dumps(info))
                archive.writestr('ets/modules.abc', b'PANDA\0\0\0')
                for name in ('libentry.so', 'liblibrustdesk.so', 'libc++_shared.so'):
                    data = bytearray(64)
                    data[:6] = b'\x7fELF\x02\x01'
                    data[18:20] = (183).to_bytes(2, 'little')
                    archive.writestr('libs/arm64-v8a/' + name, data)
            with self.assertRaisesRegex(ValueError, 'identity/version'):
                verify_ohos(hap, '1.5.14', '82', 'other.app', 'readelf', 'nm')
            with patch('verify_mobile_artifacts.run', return_value='(RUNPATH) Library runpath: [/private/build]'):
                with self.assertRaisesRegex(ValueError, 'build search paths'):
                    verify_ohos(hap, '1.5.14', '82', 'private.app', 'readelf', 'nm')

    def test_build_diagnostics_omit_private_configuration(self):
        output = safe_diagnostics(
            'note: compilation started\nerror: bad server.private.test https://server.private.test/api 192.0.2.1 secret-value',
            {'RD_ID_SERVER': 'server.private.test', 'RD_PRIVATE_API_SERVER': 'https://server.private.test/api', 'RD_TOKEN': 'secret-value'},
        )
        self.assertIn('error:', output)
        for private in ('server.private.test', '192.0.2.1', 'secret-value'):
            self.assertNotIn(private, output)


if __name__ == '__main__':
    unittest.main()
