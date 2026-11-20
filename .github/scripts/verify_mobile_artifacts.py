"""Check real mobile binaries before retaining build artifacts."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import struct
import subprocess
import zipfile


def run(*arguments):
    return subprocess.check_output(arguments, text=True, stderr=subprocess.STDOUT)


def verify_android(path, version, build_number, application_id, aapt2):
    badging = run(aapt2, 'dump', 'badging', str(path))
    package = re.search(r"^package: name='([^']+)' versionCode='([^']+)' versionName='([^']+)'", badging, re.M)
    if not package or package.groups() != (application_id, build_number, version):
        raise ValueError('APK identity/version mismatch')
    with zipfile.ZipFile(path) as archive:
        if archive.testzip() is not None:
            raise ValueError('Corrupt APK')
        native = [name for name in archive.namelist() if name.startswith('lib/') and name.endswith('.so')]
        if not native or any(not name.startswith('lib/arm64-v8a/') for name in native):
            raise ValueError('APK must contain only ARM64 native libraries')
        for required in ('librustdesk.so', 'libflutter.so', 'libapp.so', 'libc++_shared.so'):
            data = archive.read('lib/arm64-v8a/' + required)
            if data[:6] != b'\x7fELF\x02\x01' or struct.unpack_from('<H', data, 18)[0] != 183:
                raise ValueError('Invalid ARM64 ELF library: ' + required)
    return {'platform': 'android', 'architecture': 'arm64-v8a'}


def verify_ios(path, version, build_number, application_id, screen_sharing=False):
    with (path / 'Info.plist').open('rb') as stream:
        info = plistlib.load(stream)
    expected = {
        'CFBundleIdentifier': application_id,
        'CFBundleShortVersionString': version,
        'CFBundleVersion': build_number,
        'CFBundleSupportedPlatforms': ['iPhoneSimulator'],
    }
    if any(info.get(key) != value for key, value in expected.items()):
        raise ValueError('iOS simulator identity/version/platform mismatch')
    executable = path / info['CFBundleExecutable']
    binaries = [executable]
    frameworks = path / 'Frameworks'
    for framework in frameworks.glob('*.framework'):
        binary = framework / framework.stem
        if binary.is_file():
            binaries.append(binary)
    for binary in binaries:
        if run('xcrun', 'lipo', '-archs', str(binary)).strip() != 'arm64':
            raise ValueError('Simulator contains a non-ARM64 executable')
        if 'IOSSIMULATOR' not in run('xcrun', 'vtool', '-show-build', str(binary)):
            raise ValueError('Simulator bundle contains a device binary')
    symbols = run('xcrun', 'nm', '-gU', str(executable))
    required = ('session_get_rgba', 'wire_session_start', 'wire_session_login', 'wire_main_init')
    if any(not re.search(r'\b_' + symbol + r'$', symbols, re.M) for symbol in required):
        raise ValueError('Simulator executable is missing the Rust session core')
    result = {'platform': 'ios-simulator', 'architecture': 'arm64', 'physical_device_installable': False}
    if screen_sharing:
        from verify_ios_sharing import verify
        result.update(verify(path, version, build_number, application_id, symbols, run))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('platform', choices=['android', 'ios-simulator'])
    parser.add_argument('artifact', type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--build-number', required=True)
    parser.add_argument('--application-id', required=True)
    parser.add_argument('--aapt2')
    parser.add_argument('--screen-sharing', action='store_true')
    parser.add_argument('--metadata', type=Path, required=True)
    args = parser.parse_args()
    source = os.environ.get('GITHUB_SHA', '')
    if not re.fullmatch(r'[a-f0-9]{40}', source):
        raise ValueError('A full build source SHA is required')
    if args.platform == 'android':
        if not args.aapt2:
            raise ValueError('aapt2 is required for APK inspection')
        result = verify_android(args.artifact, args.version, args.build_number, args.application_id, args.aapt2)
        result.update(size=args.artifact.stat().st_size, sha256=hashlib.sha256(args.artifact.read_bytes()).hexdigest())
    else:
        result = verify_ios(args.artifact, args.version, args.build_number, args.application_id, args.screen_sharing)
    result.update(version=args.version, build_number=args.build_number, application_id=args.application_id,
                  source=source, run_id=os.environ.get('GITHUB_RUN_ID'), artifact=args.artifact.name)
    args.metadata.write_text(json.dumps(result, indent=2) + '\n')
    print('Mobile binary identity, architecture and version validated.')


if __name__ == '__main__':
    main()
