"""Install and launch the actual unsigned ARM64 simulator application."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

from mobile_build_error import safe_diagnostics


def simctl(*arguments):
    return subprocess.check_output(['xcrun', 'simctl', *arguments], text=True, stderr=subprocess.STDOUT, timeout=180)


def select_compatible_iphone(runtimes, sdk):
    suffix = '.iOS-' + sdk.replace('.', '-')
    devices = [device for runtime, items in runtimes.items() if runtime.endswith(suffix) for device in items
               if device.get('isAvailable') and device['name'].startswith('iPhone')]
    if not devices:
        raise RuntimeError('No available iPhone runtime matching build SDK')
    return devices[0]


def startup_diagnostics(udid, bundle_id):
    try:
        output = subprocess.check_output([
            'xcrun', 'simctl', 'spawn', udid, 'log', 'show', '--last', '5m', '--style', 'compact',
            '--predicate', f'process == "Runner" OR eventMessage CONTAINS "{bundle_id}"',
        ], text=True, stderr=subprocess.STDOUT, timeout=30)
        relevant = '\n'.join('error: ' + line for line in output.splitlines()
                             if re.search(r'error|fail|denied|exception|crash|dyld|Dart|Flutter|launch', line, re.I))
        diagnostic = safe_diagnostics(relevant, os.environ)
    except (subprocess.SubprocessError, OSError):
        diagnostic = 'Simulator startup diagnostics unavailable.'
    destination = Path('mobile-output/ios-startup-diagnostics.log')
    destination.parent.mkdir(exist_ok=True)
    destination.write_text(diagnostic + '\n')
    print(diagnostic)


def main():
    app, bundle_id = sys.argv[1:]
    runtimes = json.loads(simctl('list', 'devices', 'available', '--json'))['devices']
    sdk = subprocess.check_output(['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-version'], text=True, timeout=30).strip()
    device = select_compatible_iphone(runtimes, sdk)
    print(f'Launching ARM64 iPhone simulator matching SDK {sdk}.')
    udid = device['udid']
    if device['state'] != 'Booted':
        simctl('boot', udid)
    try:
        developer = os.environ.get('DEVELOPER_DIR') or subprocess.check_output(['xcode-select', '-p'], text=True, timeout=30).strip()
        subprocess.run(['open', '-a', str(Path(developer) / 'Applications/Simulator.app'),
                        '--args', '-CurrentDeviceUDID', udid], check=True, timeout=30)
        simctl('bootstatus', udid, '-b')
        simctl('install', udid, str(Path(app).resolve()))
        launched = simctl('launch', '--terminate-running-process', udid, bundle_id)
        match = re.search(r': (\d+)\s*$', launched)
        if not match:
            raise RuntimeError('Simulator did not report an application PID')
        pid = match.group(1)
        for _ in range(3):
            time.sleep(5)
            processes = simctl('spawn', udid, 'launchctl', 'list')
            if not re.search(r'^' + re.escape(pid) + r'\s+.*' + re.escape(bundle_id), processes, re.M):
                raise RuntimeError('Simulator app exited during startup')
        print('Actual iOS simulator application installed and remained running for 15 seconds; remote sessions not tested.')
    except (subprocess.SubprocessError, RuntimeError, OSError):
        startup_diagnostics(udid, bundle_id)
        raise
    finally:
        simctl('shutdown', udid)


if __name__ == '__main__':
    main()
