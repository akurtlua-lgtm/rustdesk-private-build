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
        command = ['cargo', 'test', '--locked', '--release', '-p', 'scrap', '--test', 'ios-capture-runtime-probe',
                   '--target', 'aarch64-apple-ios-sim', '--features',
                   'wayland,mobile-view-only-host']
        compilation = Path(os.environ['RUNNER_TEMP']) / 'ios-cpu-media-compile.log'
        print('Compiling native iOS integration tests before starting the execution deadline.', flush=True)
        try:
            with compilation.open('w') as stream:
                subprocess.run(command + ['--no-run'], env=environment, stdout=stream,
                               stderr=subprocess.STDOUT, timeout=5400, check=True)
        except (subprocess.TimeoutExpired, subprocess.CalledProcessError):
            print(safe_diagnostics('error: ' + compilation.read_text(errors='replace').replace('\n', '\nerror: '), environment))
            raise RuntimeError('Native iOS test compilation failed or timed out; tests not marked passed') from None
        command += ['--', 'tests::', '--nocapture']
        log = Path(os.environ['RUNNER_TEMP']) / 'ios-cpu-media-probe.log'
        print('Running the two compiled native iOS integration tests with a separate 180 second deadline.', flush=True)
        try:
            with log.open('w') as stream:
                result = subprocess.run(command, env=environment, stdout=stream, stderr=subprocess.STDOUT, timeout=180)
        except subprocess.TimeoutExpired:
            print(safe_diagnostics('error: ' + log.read_text(errors='replace').replace('\n', '\nerror: '), environment))
            raise RuntimeError('Native iOS CPU media/capture probe timed out; no passing record created') from None
        output = log.read_text(errors='replace')
        if result.returncode or not re.search(r'test result: ok\. 2 passed; 0 failed;', output):
            print(safe_diagnostics('error: ' + output.replace('\n', '\nerror: '), environment))
            raise RuntimeError('Native iOS CPU media/crypto probe did not pass exactly two tests')
        source = os.environ['GITHUB_SHA']
        if not re.fullmatch('[a-f0-9]{40}', source):
            raise ValueError('Full source SHA required')
        record = dict(source=source, run_id=os.environ['GITHUB_RUN_ID'], sdk=sdk,
                      target='aarch64-apple-ios-sim', tests=2, result='passed',
                      synthetic_bgra_vp8_box_decode_executed=True,
                      system_screen_capture_tested=False, remote_session_tested=False,
                      broadcast_extension_started=False, physical_device_tested=False)
        Path('probe-output/ios-cpu-media-probe.json').write_text(json.dumps(record, indent=2) + '\n')
        print('Two real iOS simulator CPU media and capture lifecycle tests passed; synthetic pixels only, no screen capture or network session.')
    finally:
        simctl('shutdown', udid)


if __name__ == '__main__':
    main()
