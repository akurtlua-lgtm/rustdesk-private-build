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
import tempfile
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


def verify_ios(path, version, build_number, application_id):
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
    symbols = run('xcrun', 'nm', '-g', str(executable))
    if '_session_get_rgba' not in symbols or '_dummy_method_to_enforce_bundling' not in symbols:
        raise ValueError('Simulator executable is missing the Rust session core')
    return {'platform': 'ios-simulator', 'architecture': 'arm64', 'physical_device_installable': False}


def verify_ohos(path, version, build_number, application_id, readelf, nm):
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)) or archive.testzip() is not None:
            raise ValueError('Corrupt or ambiguous HAP')
        info = json.loads(archive.read('module.json'))
        app = info['app']
        expected = {'bundleName': application_id, 'versionName': version,
                    'versionCode': int(build_number), 'minAPIVersion': 17,
                    'targetAPIVersion': 20, 'compileSdkType': 'OpenHarmony'}
        if any(app.get(key) != value for key, value in expected.items()):
            raise ValueError('HAP identity/version/API mismatch')
        if archive.read('ets/modules.abc')[:8] != b'PANDA\0\0\0':
            raise ValueError('HAP is missing compiled ArkTS bytecode')
        permissions = info['module'].get('requestPermissions', [])
        if permissions != [{'name': 'ohos.permission.INTERNET'}]:
            raise ValueError('Unexpected controller permissions')
        libraries = {name for name in names if name.endswith('.so')}
        expected_libraries = {'libs/arm64-v8a/' + name for name in
                              ('libentry.so', 'liblibrustdesk.so', 'libc++_shared.so')}
        if libraries != expected_libraries:
            raise ValueError('HAP native payload mismatch')
        dependencies = {}
        with tempfile.TemporaryDirectory() as folder:
            for name in sorted(libraries):
                if archive.getinfo(name).file_size > 256 * 1024 * 1024:
                    raise ValueError('Unexpected HAP native payload size')
                data = archive.read(name)
                if data[:6] != b'\x7fELF\x02\x01' or struct.unpack_from('<H', data, 18)[0] != 183:
                    raise ValueError('HAP contains a non-ARM64 ELF')
                binary = Path(folder) / Path(name).name
                binary.write_bytes(data)
                dynamic = run(readelf, '-d', str(binary))
                if re.search(r'\((?:RPATH|RUNPATH)\)', dynamic):
                    raise ValueError('HAP must not embed native build search paths')
                needed = set(re.findall(r'Shared library: \[([^]]+)\]', dynamic))
                dependencies[binary.name] = sorted(needed)
                if binary.name == 'liblibrustdesk.so':
                    if not needed or not needed <= {'libohaudio.so', 'libc++_shared.so', 'libc.so'}:
                        raise ValueError('OHOS core pulled in unsupported desktop libraries')
                    symbols = run(nm, '-D', '--defined-only', str(binary))
                    required = ('init', 'set_callbacks', 'version', 'free_string', 'connect',
                                'login', '2fa', 'close', 'close_all', 'mouse', 'key', 'text',
                                'view_only', 'account_request')
                    if any(not re.search(r'\brd_ohos_' + symbol + r'$', symbols, re.M) for symbol in required):
                        raise ValueError('HAP is missing the real controller/account ABI')
                if binary.name == 'libentry.so' and 'liblibrustdesk.so' not in needed:
                    raise ValueError('NAPI bridge is not linked to the Rust controller')
    return {'platform': 'ohos', 'architecture': 'arm64-v8a', 'signed': False,
            'physical_device_verified': False, 'sdk_release_type': app.get('apiReleaseType'),
            'native_dependencies': dependencies}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('platform', choices=['android', 'ios-simulator', 'ohos'])
    parser.add_argument('artifact', type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--build-number', required=True)
    parser.add_argument('--application-id', required=True)
    parser.add_argument('--aapt2')
    parser.add_argument('--readelf')
    parser.add_argument('--nm')
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
    elif args.platform == 'ohos':
        if not args.readelf or not args.nm:
            raise ValueError('SDK ELF inspection tools are required')
        result = verify_ohos(args.artifact, args.version, args.build_number, args.application_id, args.readelf, args.nm)
        result.update(size=args.artifact.stat().st_size, sha256=hashlib.sha256(args.artifact.read_bytes()).hexdigest())
    else:
        result = verify_ios(args.artifact, args.version, args.build_number, args.application_id)
    result.update(version=args.version, build_number=args.build_number, application_id=args.application_id,
                  source=source, run_id=os.environ.get('GITHUB_RUN_ID'), artifact=args.artifact.name)
    args.metadata.write_text(json.dumps(result, indent=2) + '\n')
    print('Mobile binary identity, architecture and version validated.')


if __name__ == '__main__':
    main()
