"""Execute synthetic CoreVideo conversion and authorization delivery in the simulator."""
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
    if sdk.split('.')[0] != '27':
        raise ValueError('Expected the current iOS 27 SDK')
    device = select_compatible_iphone(json.loads(simctl('list', 'devices', 'available', '--json'))['devices'], sdk)
    udid = str(uuid.UUID(device['udid']))
    if device['state'] != 'Booted':
        simctl('boot', udid)
    try:
        developer = os.environ.get('DEVELOPER_DIR') or subprocess.check_output(['xcode-select', '-p'], text=True, timeout=30).strip()
        hub = Path(developer).parent / 'Applications/DeviceHub.app'
        if not hub.is_dir():
            raise RuntimeError('Current Xcode Device Hub unavailable')
        subprocess.run(['open', '-a', str(hub)], check=True, timeout=30)
        simctl('bootstatus', udid, '-b')
        sdk_path = subprocess.check_output(['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-path'], text=True, timeout=30).strip()
        executable = Path(os.environ['RUNNER_TEMP']) / 'ios-pixel-buffer-tests'
        compilation = executable.with_suffix('.compile.log')
        command = ['xcrun', '--sdk', 'iphonesimulator', 'swiftc', '-swift-version', '6',
                   '-warnings-as-errors', '-parse-as-library', '-target', 'arm64-apple-ios27.0-simulator',
                   '-sdk', sdk_path, '-module-cache-path', str(executable.parent / 'pixel-module-cache'),
                   'res/ios-capture-frame/CaptureFrame.swift', 'res/ios-capture-frame/CaptureDelivery.swift',
                   'res/ios-capture-frame/CaptureFrameTests.swift',
                   '-o', str(executable)]
        try:
            with compilation.open('w') as log:
                subprocess.run(command, check=True, timeout=180, stdout=log, stderr=subprocess.STDOUT)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            print(safe_diagnostics('error: ' + compilation.read_text(errors='replace').replace('\n', '\nerror: '), os.environ))
            raise RuntimeError('Native iOS pixel tests did not compile') from None
        subprocess.run(['codesign', '--force', '--sign', '-', str(executable)], check=True, timeout=30)
        result = subprocess.run(['xcrun', 'simctl', 'spawn', udid, str(executable)],
                                capture_output=True, text=True, timeout=180)
        if result.returncode or result.stdout.splitlines().count('3 pixel buffer and delivery tests passed') != 1:
            print(safe_diagnostics('error: ' + (result.stdout + result.stderr).replace('\n', '\nerror: '), os.environ))
            raise RuntimeError('Native iOS pixel tests did not pass')
        source = os.environ['GITHUB_SHA']
        if not re.fullmatch('[a-f0-9]{40}', source):
            raise ValueError('Full source SHA required')
        output = Path('client-compat-output')
        output.mkdir(exist_ok=True)
        record = dict(source=source, run_id=os.environ['GITHUB_RUN_ID'], sdk=sdk,
                      architecture='arm64', result='passed', tests=3,
                      authorized_delivery_and_irreversible_stop_tested=True,
                      padded_bgra_orientation_tested=True, nv12_video_range_to_bgra_tested=True,
                      system_screen_capture_tested=False, remote_session_tested=False, physical_device_tested=False)
        (output / 'pixel-buffer.json').write_text(json.dumps(record, indent=2) + '\n')
        print('Three real iOS simulator CoreVideo conversion/delivery tests passed; synthetic pixels only.')
    finally:
        simctl('shutdown', udid)


if __name__ == '__main__':
    main()
