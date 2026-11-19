"""Install and launch the actual unsigned ARM64 simulator application."""

import json
from pathlib import Path
import re
import subprocess
import sys
import time


def simctl(*arguments):
    return subprocess.check_output(['xcrun', 'simctl', *arguments], text=True, stderr=subprocess.STDOUT, timeout=180)


def main():
    app, bundle_id = sys.argv[1:]
    runtimes = json.loads(simctl('list', 'devices', 'available', '--json'))['devices']
    devices = [device for runtime, items in runtimes.items() if '.iOS-' in runtime for device in items
               if device.get('isAvailable') and device['name'].startswith('iPhone')]
    if not devices:
        raise RuntimeError('No available iPhone simulator runtime')
    device = devices[-1]
    udid = device['udid']
    if device['state'] != 'Booted':
        simctl('boot', udid)
    try:
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
    finally:
        simctl('shutdown', udid)


if __name__ == '__main__':
    main()
