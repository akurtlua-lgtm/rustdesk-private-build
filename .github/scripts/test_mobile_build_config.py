import base64
import unittest

from mobile_build_config import validate_mobile_config


class MobileBuildConfigTests(unittest.TestCase):
    def setUp(self):
        self.config = {
            'RD_ID_SERVER': 'example.invalid',
            'RD_RELAY_SERVER': 'example.invalid',
            'RD_SERVER_PUBLIC_KEY': base64.b64encode(bytes(32)).decode(),
            'RD_PRIVATE_API_SERVER': 'https://example.invalid/private',
        }

    def test_https_api_prefix_is_supported(self):
        validate_mobile_config(self.config)

    def test_plaintext_credentials_and_url_tokens_are_refused(self):
        for api in (
            'http://example.invalid', 'https://user:password@example.invalid',
            'https://example.invalid?token=test', 'https://example.invalid#fragment',
        ):
            with self.subTest(api=api), self.assertRaises(ValueError):
                validate_mobile_config({**self.config, 'RD_PRIVATE_API_SERVER': api})

    def test_missing_or_invalid_trust_inputs_are_refused(self):
        for key in self.config:
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate_mobile_config({**self.config, key: ''})
        with self.assertRaises(ValueError):
            validate_mobile_config({**self.config, 'RD_SERVER_PUBLIC_KEY': 'invalid'})


if __name__ == '__main__':
    unittest.main()
