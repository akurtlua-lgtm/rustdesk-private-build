"""Run the actual installed sharing UI test without requesting screen recording."""
import json
import os
from pathlib import Path
import re
import subprocess
import uuid

from mobile_build_error import safe_diagnostics
from smoke_ios_simulator import select_compatible_iphone, simctl


def main():
    sdk = subprocess.check_output(['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-version'], text=True).strip()
    device = select_compatible_iphone(json.loads(simctl('list', 'devices', 'available', '--json'))['devices'], sdk)
    udid = device['udid']
    uuid.UUID(udid)
    print(f'UI simulator SDK {sdk}, device {device["name"]}, id {udid}', flush=True)
    if device['state'] != 'Booted':
        simctl('boot', udid)
    try:
        simctl('bootstatus', udid, '-b')
        log = Path(os.environ['RUNNER_TEMP']) / 'ios-sharing-ui.log'
        with log.open('w') as stream:
            result = subprocess.run(['xcodebuild', '-workspace', 'flutter/ios/Runner.xcworkspace',
                '-scheme', 'PrivateSharing', '-configuration', 'Debug', '-sdk', 'iphonesimulator',
                '-destination', 'platform=iOS Simulator,arch=arm64,id=' + udid,
                '-derivedDataPath', os.environ['RUNNER_TEMP'] + '/private-ios-derived',
                'RUSTDESK_IOS_APPLICATION_ID=' + os.environ['MOBILE_APPLICATION_ID'],
                'ARCHS=arm64', 'ONLY_ACTIVE_ARCH=YES', 'ENABLE_DEBUG_DYLIB=NO',
                'CODE_SIGNING_ALLOWED=NO', 'CODE_SIGN_ENTITLEMENTS=',
                '-only-testing:SharingUITests/SharingUITests', 'test'],
                stdout=stream, stderr=subprocess.STDOUT, timeout=600)
        output = log.read_text(errors='replace')
        if result.returncode or not re.search(r'Executed 1 test, with 0 failures', output):
            print(safe_diagnostics(output, os.environ))
            raise RuntimeError('Native iOS sharing UI test did not pass exactly one case')
        source = os.environ['GITHUB_SHA']
        if not re.fullmatch('[a-f0-9]{40}', source):
            raise ValueError('Full source SHA required')
        record = dict(source=source, run_id=os.environ['GITHUB_RUN_ID'], sdk=sdk,
            tests=1, actual_sharing_modal_opened=True, simulator_capture_disabled=True,
            system_screen_capture_tested=False, physical_device_tested=False)
        Path('mobile-output/ios-sharing-ui.json').write_text(json.dumps(record, indent=2) + '\n')
    finally:
        simctl('shutdown', udid)


if __name__ == '__main__':
    main()
