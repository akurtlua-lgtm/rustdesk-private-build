import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../common.dart';

class PrivateIosSharingButton extends StatefulWidget {
  const PrivateIosSharingButton({super.key});

  @override
  State<PrivateIosSharingButton> createState() => _PrivateIosSharingButtonState();
}

class _PrivateIosSharingButtonState extends State<PrivateIosSharingButton> {
  bool _opening = false;

  Future<void> _open() async {
    if (_opening) return;
    setState(() => _opening = true);
    try {
      await const MethodChannel('private.rustdesk/mobile-sharing')
          .invokeMethod<void>('present');
    } on PlatformException {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(translate('Failed'))));
      }
    } on MissingPluginException {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(translate('Unsupported'))));
      }
    } finally {
      if (mounted) setState(() => _opening = false);
    }
  }

  @override
  Widget build(BuildContext context) => TextButton.icon(
      onPressed: _opening ? null : _open,
      icon: const Icon(Icons.screen_share_outlined),
      label: Text(translate('Share screen')));
}
