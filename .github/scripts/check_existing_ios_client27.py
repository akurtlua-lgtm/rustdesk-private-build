"""Check the previously verified controller payload on a current iOS simulator."""

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
import zipfile

from smoke_ios_simulator import select_compatible_iphone, simctl
from verify_mobile_artifacts import verify_ios
from mobile_build_error import safe_diagnostics


CLIENT_SOURCE = '8373fa77f8a662edb15b73c5cf4eca9faee04c08'
CLIENT_RUN = '44514425331'
PAYLOAD_NAME = 'rustdesk-1.5.14-ios-simulator-arm64.app.zip'
PAYLOAD_SHA256 = '1b1e858f8c6a28f7e6787a90cb6a206b6d05e0034e99a24874629bf98889443e'
APPLICATION_ID = 'io.github.akurtlualgtm.rustdesk'


def command(*arguments):
    try:
        return subprocess.check_output(arguments, text=True, stderr=subprocess.STDOUT, timeout=180)
    except subprocess.CalledProcessError as error:
        diagnostic = '\n'.join('error: ' + line for line in error.output.splitlines())
        print(safe_diagnostics(diagnostic, os.environ), flush=True)
        raise


def main():
    payload = Path('verified-client-input') / PAYLOAD_NAME
    metadata = json.loads(payload.with_name('ios-simulator-build.json').read_text())
    expected = dict(source=CLIENT_SOURCE, run_id=CLIENT_RUN, artifact=PAYLOAD_NAME,
                    application_id=APPLICATION_ID, version='1.5.14', build_number='82',
                    platform='ios-simulator', architecture='arm64', size=64007170,
                    sha256=PAYLOAD_SHA256)
    if any(metadata.get(key) != value for key, value in expected.items()):
        raise ValueError('Controller fixture identity/source mismatch')
    if payload.stat().st_size != expected['size'] or hashlib.sha256(payload.read_bytes()).hexdigest() != PAYLOAD_SHA256:
        raise ValueError('Controller fixture payload mismatch')
    with zipfile.ZipFile(payload) as archive:
        if archive.testzip() is not None or any(Path(name).is_absolute() or '..' in Path(name).parts for name in archive.namelist()):
            raise ValueError('Invalid application archive')
    extracted = Path(os.environ['RUNNER_TEMP']) / 'verified-ios-controller'
    command('ditto', '-x', '-k', str(payload), str(extracted))
    app = extracted / 'Runner.app'
    verify_ios(app, '1.5.14', '82', APPLICATION_ID)
    sdk = command('xcrun', '--sdk', 'iphonesimulator', '--show-sdk-version').strip()
    if sdk.split('.')[0] != '27':
        raise ValueError('Expected the current iOS 27 SDK')
    devices = json.loads(simctl('list', 'devices', 'available', '--json'))['devices']
    device = select_compatible_iphone(devices, sdk)
    udid = device['udid']
    if device['state'] != 'Booted':
        simctl('boot', udid)
    try:
        developer = os.environ.get('DEVELOPER_DIR') or command('xcode-select', '-p').strip()
        device_hub = Path(developer).parent / 'Applications/DeviceHub.app'
        if not device_hub.is_dir():
            raise RuntimeError('Xcode 27 Device Hub application unavailable')
        command('open', '-a', str(device_hub))
        simctl('bootstatus', udid, '-b')
        simctl('install', udid, str(app))
        launched = simctl('launch', '--terminate-running-process', udid, APPLICATION_ID)
        match = re.search(r': (\d+)\s*$', launched)
        if not match:
            raise RuntimeError('Controller application PID unavailable')
        for _ in range(3):
            time.sleep(5)
            processes = simctl('spawn', udid, 'launchctl', 'list')
            if not re.search(r'^' + match.group(1) + r'\s+.*' + re.escape(APPLICATION_ID), processes, re.M):
                raise RuntimeError('Controller application did not remain running')
        source = os.environ['GITHUB_SHA']
        if not re.fullmatch('[a-f0-9]{40}', source):
            raise ValueError('Full verification source required')
        output = Path('client-compat-output')
        output.mkdir(exist_ok=True)
        record = dict(test_source=source, test_run=os.environ['GITHUB_RUN_ID'],
                      client_source=CLIENT_SOURCE, client_run=CLIENT_RUN,
                      client_payload_sha256=PAYLOAD_SHA256, simulator_sdk=sdk,
                      installed_and_live_seconds=15, remote_session_tested=False,
                      screen_capture_tested=False, physical_device_tested=False)
        (output / 'verification.json').write_text(json.dumps(record, indent=2) + '\n')
        print('Previously verified private controller installed and remained live for 15 seconds on iOS 27; remote sessions and screen capture not tested.')
    finally:
        simctl('shutdown', udid)


if __name__ == '__main__':
    main()
