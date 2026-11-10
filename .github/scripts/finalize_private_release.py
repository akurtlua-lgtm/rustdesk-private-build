"""Publish a complete, verified set of desktop installers as the latest release."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

from private_release import check_versions


def gh(*args):
    return subprocess.check_output(['gh', *args], text=True)


def finalize():
    root = Path(__file__).resolve().parents[2]
    version = os.environ['VERSION']
    tag = os.environ['TAG_NAME']
    repo = os.environ['GITHUB_REPOSITORY']
    check_versions(root, version, tag)
    release = json.loads(gh('api', f'repos/{repo}/releases/tags/{tag}'))
    ref = json.loads(gh('api', f'repos/{repo}/git/ref/tags/{tag}'))
    if ref['object']['sha'] != os.environ['GITHUB_SHA'] or not release['prerelease']:
        raise ValueError('Release must be an unpublished preview of this exact commit')
    expected = {f'rustdesk-{version}-{suffix}' for suffix in [
        'x86_64.exe', 'x86_64.msi', 'aarch64.dmg', 'x86_64.dmg', 'x86_64.deb',
        '0.x86_64.rpm', '0.x86_64-suse.rpm',
    ]}
    if {a['name'] for a in release['assets']} - {'SHA256SUMS'} != expected:
        raise ValueError('All seven desktop installers are required before publication')
    with tempfile.TemporaryDirectory() as directory:
        gh('release', 'download', tag, '--repo', repo, '--dir', directory, '--pattern', 'rustdesk-*')
        lines = []
        for asset in sorted(release['assets'], key=lambda item: item['name']):
            if asset['name'] not in expected:
                continue
            path = Path(directory) / asset['name']
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            if path.stat().st_size != asset['size'] or asset.get('digest') != 'sha256:' + digest:
                raise ValueError('Installer integrity mismatch: ' + asset['name'])
            lines.append(digest + '  ' + asset['name'] + '\n')
        manifest = Path(directory) / 'SHA256SUMS'
        manifest.write_text(''.join(lines))
        gh('release', 'upload', tag, str(manifest), '--repo', repo, '--clobber')
    gh('release', 'edit', tag, '--repo', repo, '--prerelease=false', '--latest',
       '--title', 'RustDesk ' + version)
    print('Published all seven verified desktop installers:', tag)


if __name__ == '__main__':
    finalize()
