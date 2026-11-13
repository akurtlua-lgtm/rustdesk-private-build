"""Sign and verify the already checked private release installers."""

from datetime import datetime, timezone
import subprocess
import sys


REPOSITORY = 'akurtlua-lgtm/rustdesk-private-build'
SIDECARS = {'rustdesk-update.json', 'rustdesk-update.json.sig'}


def prepare_private_update(root, directory, version, tag, repo):
    if repo != REPOSITORY:
        raise ValueError('Private update signing is restricted to the build repository')
    script = root / 'res/update/generate_update_metadata.py'
    metadata = directory / 'rustdesk-update.json'
    signature = directory / 'rustdesk-update.json.sig'
    common = ['--version', version, '--release-id', tag,
              '--package-id', 'rustdesk-private-build',
              '--release-base-url', 'https://github.com/' + REPOSITORY + '/releases/download']
    subprocess.run([sys.executable, str(script), 'check-key', '--rust-source',
                    str(root / 'src/update_metadata/private.rs')], check=True)
    specs = [('windows', 'x86_64', 'exe', 'x86_64.exe'),
             ('windows', 'x86_64', 'msi', 'x86_64.msi'),
             ('macos', 'aarch64', 'dmg', 'aarch64.dmg'),
             ('macos', 'x86_64', 'dmg', 'x86_64.dmg'),
             ('linux', 'x86_64', 'deb', 'x86_64.deb'),
             ('linux', 'x86_64', 'rpm', '0.x86_64.rpm'),
             ('linux', 'x86_64', 'rpm-suse', '0.x86_64-suse.rpm')]
    sign = [sys.executable, str(script), 'sign', *common,
            '--published-at', datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
            '--metadata-out', str(metadata), '--signature-out', str(signature)]
    verify = [sys.executable, str(script), 'verify', *common,
              '--metadata', str(metadata), '--signature', str(signature)]
    for platform, arch, file_format, suffix in specs:
        path = directory / f'rustdesk-{version}-{suffix}'
        sign.extend(['--artifact', platform, arch, file_format, str(path)])
        verify.extend(['--artifact', str(path)])
    subprocess.run(sign, check=True)
    subprocess.run(verify, check=True)
    return [metadata, signature]
