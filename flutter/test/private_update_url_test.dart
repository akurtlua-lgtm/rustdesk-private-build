import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/common/private_update_url.dart';

void main() {
  const prefix =
      'https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/tag/';
  test('private updates open only a stable release in our repository', () {
    expect(privateReleaseVersion('${prefix}v1.5.10'), '1.5.10');
    for (final suffix in [
      'v1.5.5-beta',
      'v01.5.5',
      'v1.5.5/extra',
      'v1.5.5?next=evil',
      'v1.5.65536',
      'v1.5.5\n'
    ]) {
      expect(privateReleaseVersion('$prefix$suffix'), isNull);
    }
    expect(privateReleaseVersion('http${prefix.substring(5)}v1.5.5'), isNull);
    expect(
        privateReleaseVersion(
            'https://github.com/rustdesk/rustdesk/releases/tag/v1.5.5'),
        isNull);
  });
}
