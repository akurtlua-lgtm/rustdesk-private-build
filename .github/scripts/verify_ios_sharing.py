"""Inspect the opt-in native iOS sharing application and its real broadcast extension."""
import plistlib
import re


def verify(path, version, build_number, application_id, app_symbols, run):
    extension = path / 'PlugIns/BroadcastShare.appex'
    with (extension / 'Info.plist').open('rb') as stream:
        info = plistlib.load(stream)
    display_name = info.get('CFBundleDisplayName')
    if not isinstance(display_name, str) or not display_name.strip():
        raise ValueError('Missing sharing extension display name')
    expected = {'CFBundleIdentifier': application_id + '.BroadcastShare',
                'CFBundleShortVersionString': version, 'CFBundleVersion': build_number,
                'CFBundleSupportedPlatforms': ['iPhoneSimulator']}
    if any(info.get(key) != value for key, value in expected.items()):
        raise ValueError('Sharing extension identity/version/platform mismatch')
    if info.get('NSExtension', {}).get('NSExtensionPointIdentifier') != 'com.apple.broadcast-services-upload':
        raise ValueError('Missing broadcast upload capability')
    with (path / 'Info.plist').open('rb') as stream:
        app = plistlib.load(stream)
    for record in (app, info):
        if record.get('RustDeskSharingAppGroup') != 'group.' + application_id + '.sharing':
            raise ValueError('Sharing app group mismatch')
        if not record.get('RustDeskSharingKeychainGroup', '').endswith(application_id + '.sharing'):
            raise ValueError('Sharing Keychain group mismatch')
    executable = extension / info['CFBundleExecutable']
    if run('xcrun', 'lipo', '-archs', str(executable)).strip() != 'arm64':
        raise ValueError('Sharing extension must be ARM64')
    if not re.search(r'^\s*platform\s+IOSSIMULATOR\s*$', run('xcrun', 'vtool', '-show-build', str(executable)), re.M):
        raise ValueError('Sharing extension SDK mismatch')
    extension_symbols = run('xcrun', 'nm', '-gU', str(executable))
    host = ('initialize', 'set_password', 'configure_servers', 'id', 'begin', 'frame',
            'stop', 'state', 'run', 'release', 'legacy_import')
    bootstrap = ('set_password', 'servers', 'legacy_policy')
    for symbols, required in ((extension_symbols, host), (app_symbols, bootstrap)):
        if any(not re.search(r'\b_rd_mobile_host_' + name + r'$', symbols, re.M) for name in required):
            raise ValueError('Sharing process is missing its actual native host ABI')
    dependencies = run('xcrun', 'otool', '-L', str(executable)) + run('xcrun', 'nm', '-u', str(executable))
    if re.search(r'Flutter|Dart_|UIApplication|NSApplication|/PrivateFrameworks/', dependencies):
        raise ValueError('Broadcast extension depends on application-only APIs')
    return {'screen_sharing_enabled': True, 'system_screen_capture_verified': False,
            'physical_device_verified': False, 'broadcast_extension_inspected': True}
