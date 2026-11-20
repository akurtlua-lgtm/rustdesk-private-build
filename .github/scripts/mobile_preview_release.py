"""Sign the verified main-branch Android payload and publish a separate mobile preview."""

import base64
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import zipfile

from verify_mobile_artifacts import verify_android

REPOSITORY = 'akurtlua-lgtm/rustdesk-private-build'
SIGNATURE_ENTRY = re.compile(r'^META-INF/(?:MANIFEST\.MF|[^/]+\.(?:SF|RSA|DSA|EC))$', re.I)


def check_record(record, artifact, version, build, application_id, source, run_id):
    expected = {'version': version, 'build_number': build, 'application_id': application_id,
                'source': source, 'run_id': run_id, 'artifact': artifact.name}
    if any(record.get(key) != value for key, value in expected.items()):
        raise ValueError('Mobile artifact source/identity mismatch')
    if record.get('size') != artifact.stat().st_size or record.get('sha256') != digest(artifact):
        raise ValueError('Mobile artifact payload digest mismatch')


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def apk_payload(path):
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)) or archive.testzip() is not None:
            raise ValueError('Corrupt or ambiguous APK')
        return {name: hashlib.sha256(archive.read(name)).hexdigest() for name in names
                if not SIGNATURE_ENTRY.fullmatch(name)}


def execute(*arguments):
    return subprocess.check_output(arguments, text=True, stderr=subprocess.STDOUT)


def gh_api(path, method='GET', body=None):
    command = ['gh', 'api', f'repos/{REPOSITORY}/{path}', '--method', method]
    if body is None:
        return json.loads(execute(*command))
    with tempfile.TemporaryDirectory() as directory:
        payload = Path(directory) / 'request.json'
        payload.write_text(json.dumps(body))
        return json.loads(execute(*command, '--input', str(payload)))


def main():
    environment = os.environ
    if (environment.get('GITHUB_REPOSITORY') != REPOSITORY
            or environment.get('GITHUB_REF') != 'refs/heads/main'
            or environment.get('GITHUB_EVENT_NAME') != 'workflow_dispatch'):
        raise ValueError('Mobile preview signing is restricted to main manual runs')
    signing_key = environment.pop('RD_PRIVATE_ANDROID_KEYSTORE_BASE64')
    signing_password = environment.pop('RD_PRIVATE_ANDROID_KEYSTORE_PASSWORD')
    source = environment['GITHUB_SHA']
    run_id = environment['GITHUB_RUN_ID']
    version = environment['VERSION']
    build = environment['BUILD_NUMBER']
    application_id = environment['MOBILE_APPLICATION_ID']
    if not re.fullmatch(r'[a-f0-9]{40}', source) or not re.fullmatch(r'\d+\.\d+\.\d+', version):
        raise ValueError('Invalid release identity')
    root = Path('mobile-preview-input')
    android = root / f'rustdesk-{version}-android-arm64-unsigned.apk'
    ios = root / f'rustdesk-{version}-ios-simulator-arm64.app.zip'
    for name, artifact, platform in [('android-build.json', android, 'android'),
                                      ('ios-simulator-build.json', ios, 'ios-simulator')]:
        record = json.loads((root / name).read_text())
        check_record(record, artifact, version, build, application_id, source, run_id)
        if record.get('platform') != platform:
            raise ValueError('Wrong mobile platform metadata')
    info = gh_api(f'actions/runs/{run_id}')
    if info['head_sha'] != source or info['head_branch'] != 'main':
        raise ValueError('Mobile run source is not the signing source')
    jobs = gh_api(f'actions/runs/{run_id}/jobs?per_page=100')['jobs']
    for name in ['mobile-config', 'android-arm64', 'ios-simulator-arm64']:
        matching = [job for job in jobs if job['name'] == name]
        if len(matching) != 1 or matching[0]['conclusion'] != 'success':
            raise ValueError('Required mobile build or simulator test did not pass')
    aapt2 = environment['AAPT2']
    apksigner = environment['APKSIGNER']
    verify_android(android, version, build, application_id, aapt2)
    expected_certificate = Path('res/android/private-release-certificate.sha256').read_text().strip()
    if not re.fullmatch(r'[a-f0-9]{64}', expected_certificate):
        raise ValueError('Invalid pinned Android release certificate')
    tag = f'mobile-preview-v{version}'
    # A preview must not replace a prior release or move its tag.
    tags = gh_api(f'git/matching-refs/tags/{tag}')
    if any(item['ref'] == f'refs/tags/{tag}' for item in tags):
        raise ValueError('Mobile preview tag already exists')
    output = Path('mobile-preview-output')
    output.mkdir(exist_ok=False)
    signed = output / f'rustdesk-{version}-android-arm64.apk'
    with tempfile.TemporaryDirectory(dir=environment['RUNNER_TEMP']) as directory:
        folder = Path(directory)
        key = folder / 'signing.p12'
        password = folder / 'password.txt'
        key.touch(mode=0o600)
        password.touch(mode=0o600)
        key.write_bytes(base64.b64decode(signing_key, validate=True))
        value = signing_password
        if not value or '\n' in value or '\r' in value:
            raise ValueError('Invalid Android signing password format')
        password.write_text(value)
        execute(apksigner, 'sign', '--ks', str(key), '--ks-key-alias', 'rustdesk-private',
                '--ks-pass', 'file:' + str(password), '--key-pass', 'file:' + str(password),
                '--v1-signing-enabled', 'true', '--v2-signing-enabled', 'true',
                '--v3-signing-enabled', 'true', '--v4-signing-enabled', 'false',
                '--out', str(signed), str(android))
    verification = execute(apksigner, 'verify', '--verbose', '--print-certs', str(signed))
    certificates = re.findall(r'^Signer #\d+ certificate SHA-256 digest: ([a-f0-9]+)$', verification, re.M | re.I)
    if [value.lower() for value in certificates] != [expected_certificate]:
        raise ValueError('Signed APK does not match the persistent release identity')
    if apk_payload(android) != apk_payload(signed):
        raise ValueError('Signing altered the compiled Android payload')
    verify_android(signed, version, build, application_id, aapt2)
    copied_ios = output / ios.name
    with ios.open('rb') as stream, copied_ios.open('wb') as target:
        shutil.copyfileobj(stream, target)
    if digest(copied_ios) != digest(ios):
        raise ValueError('Simulator archive changed while staging')
    assets = [signed, copied_ios]
    sums = output / 'SHA256SUMS'
    sums.write_text(''.join(f'{digest(path)}  {path.name}\n' for path in assets))
    metadata = output / 'MOBILE_BUILD.json'
    metadata.write_text(json.dumps({'version': version, 'build_number': build, 'source': source,
                                   'run_id': run_id, 'application_id': application_id,
                                   'android_certificate_sha256': expected_certificate,
                                   'android_physical_device_verified': False,
                                   'ios': 'ARM64 simulator app; cannot install on a physical phone',
                                   'ohos_included': False}, indent=2) + '\n')
    assets += [sums, metadata]
    previous_latest = gh_api('releases/latest')['id']
    gh_api('git/refs', 'POST', {'ref': 'refs/tags/' + tag, 'sha': source})
    release = gh_api('releases', 'POST', {
        'tag_name': tag, 'target_commitish': source, 'name': f'Mobile development preview {version}',
        'draft': True, 'prerelease': True, 'make_latest': 'false',
        'body': ('Android ARM64 installer signed with the persistent private-build certificate. '
                 'Android/compatible HarmonyOS physical-device permission and remote-session validation is pending. '
                 'The iOS archive is simulator-only, not a phone installer. Native HarmonyOS is not included. '
                 'This preview does not replace the stable desktop release.\n\nSource: ' + source),
    })
    subprocess.run(['gh', 'release', 'upload', tag, '--repo', REPOSITORY, *map(str, assets)], check=True)
    remote = gh_api(f'releases/{release["id"]}')
    uploaded = {item['name']: item for item in remote['assets']}
    if set(uploaded) != {path.name for path in assets}:
        raise ValueError('Incomplete mobile preview upload; keep draft')
    for path in assets:
        item = uploaded[path.name]
        if item['size'] != path.stat().st_size or item.get('digest') != 'sha256:' + digest(path):
            raise ValueError('Uploaded mobile digest mismatch; keep draft')
    ref = gh_api(f'git/ref/tags/{tag}')
    if ref['object']['sha'] != source or ref['object']['type'] != 'commit':
        raise ValueError('Mobile preview tag does not identify verified source; keep draft')
    if gh_api('releases/latest')['id'] != previous_latest:
        raise ValueError('Stable desktop latest changed; keep preview draft')
    gh_api(f'releases/{release["id"]}', 'PATCH', {'draft': False, 'prerelease': True, 'make_latest': 'false'})
    if gh_api('releases/latest')['id'] != previous_latest:
        raise ValueError('Stable desktop latest unexpectedly changed')
    print('Separate mobile preview published; persistent APK certificate and payload verified.')


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError:
        raise SystemExit('Mobile signing/publication tool failed; private tool output omitted.') from None
