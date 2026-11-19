"""Validate private mobile build inputs without printing their values."""

import base64
import binascii
import os
from urllib.parse import urlsplit


def validate_mobile_config(environment):
    for key in (
        'RD_ID_SERVER', 'RD_RELAY_SERVER', 'RD_SERVER_PUBLIC_KEY',
        'RD_PRIVATE_API_SERVER',
    ):
        value = environment.get(key, '')
        if not value or value.strip() != value or '\n' in value or '\r' in value:
            raise ValueError('Missing or malformed private mobile configuration: ' + key)
    try:
        public_key = base64.b64decode(environment['RD_SERVER_PUBLIC_KEY'], validate=True)
    except (ValueError, binascii.Error):
        raise ValueError('Malformed private server public key') from None
    if len(public_key) != 32:
        raise ValueError('Private server public key must contain 32 bytes')
    try:
        api = urlsplit(environment['RD_PRIVATE_API_SERVER'])
        valid = (
            api.scheme == 'https' and api.hostname and api.username is None
            and api.password is None and not api.query and not api.fragment
        )
        api.port
    except ValueError:
        valid = False
    if not valid:
        raise ValueError('Private mobile API must be a credential-free HTTPS base URL')


if __name__ == '__main__':
    validate_mobile_config(os.environ)
    print('Private mobile HTTPS/server configuration validated; values omitted.')
