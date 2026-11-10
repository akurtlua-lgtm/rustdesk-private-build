"""Check version consistency and refuse an already-published version."""

import os
from pathlib import Path
import re
import subprocess


def version_number(value):
    if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', value):
        raise ValueError('Version must use major.minor.patch without a suffix')
    parts = tuple(map(int, value.split('.')))
    if max(parts) > 65535 or max(parts[:2]) > 255:
        raise ValueError('Version exceeds Windows installer limits')
    return parts


def check_versions(root, expected, tag):
    version_number(expected)
    if tag != 'v' + expected:
        raise ValueError('Release tag must be v + VERSION')
    cargo = re.search(r'^version = "([^"]+)"$', (root / 'Cargo.toml').read_text(), re.M)
    lock = re.search(r'\[\[package\]\]\nname = "rustdesk"\nversion = "([^"]+)"',
                     (root / 'Cargo.lock').read_text())
    packer = re.search(r'^version = "([^"]+)"$',
                       (root / 'libs/portable/Cargo.toml').read_text(), re.M)
    packer_lock = re.search(r'\[\[package\]\]\nname = "rustdesk-portable-packer"\nversion = "([^"]+)"',
                            (root / 'Cargo.lock').read_text())
    flutter = re.search(r'^version: ([^+\s]+)\+([0-9]+)$',
                        (root / 'flutter/pubspec.yaml').read_text(), re.M)
    rpms = [re.search(r'^Version:\s*(\S+)\s*$', (root / name).read_text(), re.M)
            for name in ['res/rpm-flutter.spec', 'res/rpm-flutter-suse.spec']]
    if not cargo or not lock or not packer or not packer_lock or not flutter or not all(rpms) or not 0 < int(flutter[2]) <= 65535:
        raise ValueError('Missing program version or Flutter build number')
    if any(value != expected for value in [cargo[1], lock[1], packer[1], packer_lock[1], flutter[1], *[rpm[1] for rpm in rpms]]):
        raise ValueError('Cargo, portable packer, lockfile, Flutter, RPM, and build VERSION must match')


def check_new_version(expected, remote_tags):
    previous = []
    for line in remote_tags.splitlines():
        ref = line.split()[-1]
        match = re.fullmatch(r'refs/tags/(?:v|private-)([0-9]+\.[0-9]+\.[0-9]+)(?:-[A-Za-z0-9.-]+)?', ref)
        if match:
            previous.append(version_number(match[1]))
    if previous and version_number(expected) <= max(previous):
        raise ValueError('Increment VERSION before creating another release')


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[2]
    expected = os.environ['VERSION']
    check_versions(root, expected, os.environ['TAG_NAME'])
    tags = subprocess.check_output(
        ['git', 'ls-remote', '--tags', '--refs', 'origin'], cwd=root, text=True)
    check_new_version(expected, tags)
    print('Version consistency and monotonic release checks passed:', expected)
