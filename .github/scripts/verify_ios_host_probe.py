"""Inspect a development-only extension; never install or publish it."""

import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import sys


def main():
    bundle = Path(sys.argv[1])
    sdk = sys.argv[2]
    platform = {'iphoneos': 'IOS', 'iphonesimulator': 'IOSSIMULATOR'}[sdk]
    with (bundle / 'Info.plist').open('rb') as stream:
        info = plistlib.load(stream)
    if info['CFBundleIdentifier'] != 'io.github.akurtlualgtm.rustdesk.broadcast-probe':
        raise ValueError('Unexpected extension identity')
    if info['NSExtension']['NSExtensionPointIdentifier'] != 'com.apple.broadcast-services-upload':
        raise ValueError('Missing real broadcast upload extension')
    binary = bundle / info['CFBundleExecutable']
    def run(*args):
        return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT)
    if run('xcrun', 'lipo', '-archs', str(binary)).strip() != 'arm64':
        raise ValueError('Non-ARM64 extension')
    if not re.search(r'^\s*platform\s+' + platform + r'\s*$', run('xcrun', 'vtool', '-show-build', str(binary)), re.M):
        raise ValueError('Extension SDK/platform mismatch')
    defined = run('xcrun', 'nm', '-gU', str(binary))
    if not re.search(r'\b_rd_ios_host_link_probe$', defined, re.M):
        raise ValueError('Missing real Rust crypto probe')
    dependencies = run('xcrun', 'otool', '-L', str(binary))
    undefined = run('xcrun', 'nm', '-u', str(binary))
    if re.search(r'Flutter|Dart_|UIApplication|NSApplication|/PrivateFrameworks/', dependencies + undefined):
        raise ValueError('Extension still depends on Flutter or application-only APIs')
    source = os.environ['GITHUB_SHA']
    if not re.fullmatch('[a-f0-9]{40}', source):
        raise ValueError('Full source SHA required')
    archive = Path('probe-output') / f'ios-host-link-probe-{sdk}.appex.zip'
    subprocess.run(['ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', str(bundle), str(archive)], check=True)
    record = dict(source=source, run_id=os.environ['GITHUB_RUN_ID'], sdk=sdk, architecture='arm64',
                  artifact=archive.name, size=archive.stat().st_size,
                  sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                  signed=False, installable=False, screen_sharing_implemented=False,
                  runtime_tested=False, configuration='synthetic example.invalid only')
    archive.with_suffix('.json').write_text(json.dumps(record, indent=2) + '\n')
    print('Real ARM64 broadcast extension linked without Flutter; development probe only, not screen sharing.')


if __name__ == '__main__':
    main()
