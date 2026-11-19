"""Execute the offline CPU media/crypto integration test inside the iOS simulator."""

import json
import os
from pathlib import Path
import re
import subprocess
import uuid

from mobile_build_error import safe_diagnostics
from smoke_ios_simulator import select_compatible_iphone, simctl


def main():
    sdk = subprocess.check_output(['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-version'], text=True, timeout=30).strip()
    devices = json.loads(simctl('list', 'devices', 'available', '--json'))['devices']
    device = select_compatible_iphone(devices, sdk)
    udid = str(uuid.UUID(device['udid']))
    if device['state'] != 'Booted':
        simctl('boot', udid)
    try:
        subprocess.run(['open', '-a', str(Path(os.environ['DEVELOPER_DIR']) / 'Applications/Simulator.app'),
                        '--args', '-CurrentDeviceUDID', udid], check=True, timeout=30)
        simctl('bootstatus', udid, '-b')
        environment = dict(os.environ, CARGO_TARGET_AARCH64_APPLE_IOS_SIM_RUNNER=f'xcrun simctl spawn {udid}')
        command = ['cargo', 'test', '--locked', '--release', '--test', 'ios-capture-runtime-probe',
                   '--target', 'aarch64-apple-ios-sim', '--features',
                   'flutter,ios-host-link-probe,mobile-view-only-host', '--', '--exact',
                   'tests::synthetic_bgra_vp8_authenticated_roundtrip', '--nocapture']
        log = Path(os.environ['RUNNER_TEMP']) / 'ios-cpu-media-probe.log'
        with log.open('w') as stream:
            result = subprocess.run(command, env=environment, stdout=stream, stderr=subprocess.STDOUT, timeout=1200)
        output = log.read_text(errors='replace')
        if result.returncode or not re.search(r'test result: ok\. 1 passed; 0 failed;', output):
            print(safe_diagnostics('error: ' + output.replace('\n', '\nerror: '), environment))
            raise RuntimeError('Native iOS CPU media/crypto probe did not pass exactly one test')
        source = os.environ['GITHUB_SHA']
        if not re.fullmatch('[a-f0-9]{40}', source):
            raise ValueError('Full source SHA required')
        record = dict(source=source, run_id=os.environ['GITHUB_RUN_ID'], sdk=sdk,
                      target='aarch64-apple-ios-sim', tests=1, result='passed',
                      synthetic_bgra_vp8_box_decode_executed=True,
                      system_screen_capture_tested=False, remote_session_tested=False,
                      broadcast_extension_started=False, physical_device_tested=False)
        Path('probe-output/ios-cpu-media-probe.json').write_text(json.dumps(record, indent=2) + '\n')
        print('One real iOS simulator CPU BGRA/VP8/authenticated Box/decode test passed; synthetic pixels only, no screen capture or network session.')
    finally:
        simctl('shutdown', udid)


if __name__ == '__main__':
    main()
