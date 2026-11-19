"""Print bounded compiler diagnostics without private build configuration."""

import os
from pathlib import Path
import re
import sys


def safe_diagnostics(content, environment):
    for key, value in environment.items():
        if value and (key.startswith('RD_') or key in ('GH_TOKEN', 'GITHUB_TOKEN')):
            content = content.replace(value, '[REDACTED]')
    content = re.sub(r'https?://[^\s\"\'<>]+', '[URL]', content)
    content = re.sub(r'\b(?:\d{1,3}\.){3}\d{1,3}\b', '[IP]', content)
    lines = content.splitlines()
    relevant = [line[:2000] for line in lines if re.search(r'error:|undefined symbol|ld:|failed|fatal:', line, re.I)]
    return '\n'.join(relevant[-40:]) or 'Build failed; detailed log remains only in runner temporary storage.'


if __name__ == '__main__':
    print(safe_diagnostics(Path(sys.argv[1]).read_text(errors='replace'), os.environ))
