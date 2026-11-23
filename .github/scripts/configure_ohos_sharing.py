"""Select the explicit native sharing build variant in an isolated checkout."""
import json
from pathlib import Path
import sys


def configure(root, enabled):
    manifest = root / 'harmonyos/entry/src/main/module.json5'
    info = json.loads(manifest.read_text())
    module = info['module']
    module['requestPermissions'] = [{'name': 'ohos.permission.INTERNET'}]
    ability = module['abilities'][0]
    if enabled:
        module['requestPermissions'].append({'name': 'ohos.permission.KEEP_BACKGROUND_RUNNING'})
        ability['backgroundModes'] = ['audioRecording']
    else:
        ability.pop('backgroundModes', None)
    manifest.write_text(json.dumps(info, indent=2) + '\n')
    profile = root / 'harmonyos/entry/build-profile.json5'
    info = json.loads(profile.read_text())
    info['buildOption']['externalNativeOptions']['arguments'] = (
        '-DRD_ENABLE_MOBILE_SCREEN_SHARING=' + ('ON' if enabled else 'OFF'))
    profile.write_text(json.dumps(info, indent=2) + '\n')


if __name__ == '__main__':
    if len(sys.argv) != 2 or sys.argv[1] not in ('true', 'false'):
        raise ValueError('Expected an explicit sharing build selection')
    configure(Path.cwd(), sys.argv[1] == 'true')
