String? privateReleaseVersion(String url) {
  final match = RegExp(
          r'^https://github\.com/akurtlua-lgtm/rustdesk-private-build/releases/tag/v((?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*))$')
      .firstMatch(url);
  final value = match?.group(1);
  if (value == null ||
      match!.end != url.length ||
      value.split('.').any((part) => (int.tryParse(part) ?? 65536) > 65535)) {
    return null;
  }
  return value;
}
