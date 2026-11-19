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
        host_command = ['cargo', 'test', '--locked', '--release', '--lib',
                        '--target', 'aarch64-apple-ios-sim', '--features',
                        'flutter,ios-host-link-probe,mobile-view-only-host,ios-hostcore-compile-probe']
        host_compile = Path(os.environ['RUNNER_TEMP']) / 'ios-host-loopback-compile.log'
        print('Linking root library tests for the real encrypted host loopback.', flush=True)
        try:
            with host_compile.open('w') as stream:
                subprocess.run(host_command + ['--no-run'], env=environment, stdout=stream,
                               stderr=subprocess.STDOUT, timeout=5400, check=True)
        except (subprocess.TimeoutExpired, subprocess.CalledProcessError):
            print(safe_diagnostics('error: ' + host_compile.read_text(errors='replace').replace('\n', '\nerror: '), environment))
            raise RuntimeError('Root iOS host tests did not link; no loopback pass claimed') from None
        host_log = Path(os.environ['RUNNER_TEMP']) / 'ios-host-loopback.log'
        try:
            with host_log.open('w') as stream:
                host_result = subprocess.run(host_command + ['--', 'client::mobile_host_tests::',
                                            '--test-threads=1', '--nocapture'], env=environment,
                                            stdout=stream, stderr=subprocess.STDOUT, timeout=180)
        except subprocess.TimeoutExpired:
            print(safe_diagnostics('error: ' + host_log.read_text(errors='replace').replace('\n', '\nerror: '), environment))
            raise RuntimeError('Real iOS host loopback timed out; no passing record') from None
        host_output = host_log.read_text(errors='replace')
        if host_result.returncode or not re.search(r'test result: ok\. 2 passed; 0 failed;', host_output):
            print(safe_diagnostics('error: ' + host_output.replace('\n', '\nerror: '), environment))
            raise RuntimeError('Real iOS host loopback did not pass exactly two tests')
        source = os.environ['GITHUB_SHA']
        if not re.fullmatch('[a-f0-9]{40}', source):
            raise ValueError('Full source SHA required')
        record = dict(source=source, run_id=os.environ['GITHUB_RUN_ID'], sdk=sdk,
                      target='aarch64-apple-ios-sim', tests=2, result='passed',
                      synthetic_bgra_vp8_box_decode_executed=True,
                      host_loopback_tests=2, real_encrypted_host_loopback=True,
                      device_password_totp_read_only_and_scope_revocation=True,
                      system_screen_capture_tested=False, remote_session_tested=False,
                      broadcast_extension_started=False, physical_device_tested=False)
        Path('probe-output/ios-cpu-media-probe.json').write_text(json.dumps(record, indent=2) + '\n')
        print('Two media tests and two real encrypted TCP host loopback tests passed; synthetic pixels and isolated test configuration, no system capture or physical device.')
    finally:
        simctl('shutdown', udid)


if __name__ == '__main__':
    main()
