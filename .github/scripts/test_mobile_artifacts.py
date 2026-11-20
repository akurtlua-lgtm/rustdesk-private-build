from pathlib import Path
import plistlib
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from mobile_build_error import safe_diagnostics
from smoke_ios_simulator import select_compatible_iphone
from verify_mobile_artifacts import verify_android, verify_ios


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

    def test_ios_sharing_inspects_real_extension_and_rejects_missing_auth_core(self):
        from verify_ios_sharing import verify
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            extension = app / 'PlugIns/BroadcastShare.appex'
            extension.mkdir(parents=True)
            groups = dict(RustDeskSharingAppGroup='group.private.app.sharing',
                          RustDeskSharingKeychainGroup='private.app.sharing')
            (app / 'Info.plist').write_bytes(plistlib.dumps(groups))
            info = dict(groups, CFBundleIdentifier='private.app.BroadcastShare', CFBundleExecutable='BroadcastShare',
                        CFBundleShortVersionString='1.5.14', CFBundleVersion='82',
                        CFBundleSupportedPlatforms=['iPhoneSimulator'],
                        NSExtension=dict(NSExtensionPointIdentifier='com.apple.broadcast-services-upload'))
            (extension / 'Info.plist').write_bytes(plistlib.dumps(info))
            required = ('initialize', 'set_password', 'configure_servers', 'servers', 'id', 'begin',
                        'frame', 'stop', 'state', 'run', 'release', 'legacy_policy', 'legacy_import')
            symbols = '\n'.join('T _rd_mobile_host_' + name for name in required)
            app_symbols = symbols
            dependencies = 'libSystem.B.dylib'
            def inspect(*args):
                return {'lipo': 'arm64', 'vtool': ' platform IOSSIMULATOR', 'nm': symbols,
                        'otool': dependencies}[args[1]]
            self.assertTrue(verify(app, '1.5.14', '82', 'private.app', app_symbols, inspect)['broadcast_extension_inspected'])
            symbols = symbols.replace('_rd_mobile_host_run', '_missing_run')
            with self.assertRaisesRegex(ValueError, 'actual native host ABI'):
                verify(app, '1.5.14', '82', 'private.app', app_symbols, inspect)
            symbols = '\n'.join('T _rd_mobile_host_' + name for name in required)
            dependencies = 'Flutter.framework/Flutter'
            with self.assertRaisesRegex(ValueError, 'application-only APIs'):
                verify(app, '1.5.14', '82', 'private.app', app_symbols, inspect)

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
