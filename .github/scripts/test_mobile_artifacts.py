from pathlib import Path
import json
import plistlib
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from mobile_build_error import safe_diagnostics
from smoke_ios_simulator import select_compatible_iphone
from verify_mobile_artifacts import verify_android, verify_ios, verify_ohos


class MobileArtifactTests(unittest.TestCase):
    def test_simulator_matches_build_sdk_and_refuses_other_runtimes(self):
        runtimes = {
            'com.apple.CoreSimulator.SimRuntime.iOS-18-5': [{'isAvailable': True, 'name': 'iPhone 16', 'udid': 'supported'}],
            'com.apple.CoreSimulator.SimRuntime.iOS-26-0': [{'isAvailable': True, 'name': 'iPhone 17', 'udid': 'unsupported'}],
        }
        self.assertEqual(select_compatible_iphone(runtimes, '18.5')['udid'], 'supported')
        with self.assertRaisesRegex(RuntimeError, 'matching build SDK'):
            select_compatible_iphone(runtimes, '19.0')

    def test_ios_requires_real_rust_exports_not_static_header_helper(self):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            info = dict(CFBundleIdentifier='private.app', CFBundleExecutable='Runner',
                        CFBundleShortVersionString='1.5.14', CFBundleVersion='82',
                        CFBundleSupportedPlatforms=['iPhoneSimulator'])
            (app / 'Info.plist').write_bytes(plistlib.dumps(info))
            symbols = '\n'.join('00000001 T _' + symbol for symbol in
                                ('session_get_rgba', 'wire_session_start', 'wire_session_login', 'wire_main_init'))
            def inspect(*args):
                return {'lipo': 'arm64', 'vtool': 'platform IOSSIMULATOR', 'nm': symbols}[args[1]]
            with patch('verify_mobile_artifacts.run', side_effect=inspect):
                self.assertEqual(verify_ios(app, '1.5.14', '82', 'private.app')['platform'], 'ios-simulator')
                symbols = symbols.replace('_wire_session_login', '_missing_login')
                with self.assertRaisesRegex(ValueError, 'Rust session core'):
                    verify_ios(app, '1.5.14', '82', 'private.app')

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

    def test_hap_sharing_requires_exact_permissions_and_host_abi(self):
        with tempfile.TemporaryDirectory() as directory:
            hap = Path(directory) / 'sharing.hap'
            info = {'app': {'bundleName': 'private.app', 'versionName': '1.5.14',
                            'versionCode': 82, 'minAPIVersion': 17,
                            'targetAPIVersion': 20, 'compileSdkType': 'OpenHarmony'},
                    'module': {'abilities': [{'name': 'EntryAbility', 'backgroundModes': ['audioRecording']}],
                               'requestPermissions': [{'name': 'ohos.permission.INTERNET'},
                                                      {'name': 'ohos.permission.KEEP_BACKGROUND_RUNNING'}]}}
            def write_hap():
                with zipfile.ZipFile(hap, 'w') as archive:
                    archive.writestr('module.json', json.dumps(info))
                    archive.writestr('ets/modules.abc', b'PANDA\0\0\0')
                    elf = bytearray(64)
                    elf[:6] = b'\x7fELF\x02\x01'
                    elf[18:20] = (183).to_bytes(2, 'little')
                    for name in ('libentry.so', 'liblibrustdesk.so', 'libc++_shared.so'):
                        archive.writestr('libs/arm64-v8a/' + name, elf)
            write_hap()
            controller = ('init', 'set_callbacks', 'version', 'free_string', 'connect', 'login',
                          '2fa', 'close', 'close_all', 'mouse', 'key', 'text', 'view_only', 'account_request')
            host = ('initialize', 'set_password', 'configure_servers', 'servers', 'id', 'begin',
                    'frame', 'stop', 'state', 'run', 'release')
            symbols = '\n'.join('T rd_ohos_' + name for name in controller)
            symbols += '\n' + '\n'.join('T rd_mobile_host_' + name for name in host)
            def inspect(tool, *args):
                return symbols if tool == 'nm' else '(NEEDED) Shared library: [liblibrustdesk.so]' if args[-1].endswith('libentry.so') else '(NEEDED) Shared library: [libc.so]'
            with patch('verify_mobile_artifacts.run', side_effect=inspect):
                with self.assertRaisesRegex(ValueError, 'permissions'):
                    verify_ohos(hap, '1.5.14', '82', 'private.app', 'readelf', 'nm')
                self.assertTrue(verify_ohos(hap, '1.5.14', '82', 'private.app', 'readelf', 'nm', True)['screen_sharing_enabled'])
                symbols = symbols.replace('rd_mobile_host_stop', 'missing_stop')
                with self.assertRaisesRegex(ValueError, 'sharing ABI'):
                    verify_ohos(hap, '1.5.14', '82', 'private.app', 'readelf', 'nm', True)
                info['module']['requestPermissions'].append({'name': 'ohos.permission.MICROPHONE'})
                write_hap()
                with self.assertRaisesRegex(ValueError, 'permissions'):
                    verify_ohos(hap, '1.5.14', '82', 'private.app', 'readelf', 'nm', True)

    def test_build_diagnostics_omit_private_configuration(self):
        output = safe_diagnostics(
            'note: compilation started\nerror: bad server.private.test https://server.private.test/api 192.0.2.1 secret-value',
            {'RD_ID_SERVER': 'server.private.test:21116', 'RD_PRIVATE_API_SERVER': 'https://server.private.test/api', 'RD_TOKEN': 'secret-value'},
        )
        self.assertIn('error:', output)
        for private in ('server.private.test', '192.0.2.1', 'secret-value'):
            self.assertNotIn(private, output)
        for invalid in ('https://[bad', 'https://', 'https:///server.private.test'):
            self.assertNotIn('server.private.test', safe_diagnostics(
                'error: failed https://server.private.test/account', {'RD_ID_SERVER': invalid}))


if __name__ == '__main__':
    unittest.main()
