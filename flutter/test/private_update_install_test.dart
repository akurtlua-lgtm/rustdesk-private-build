import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/desktop/widgets/update_progress.dart';
import 'package:flutter_hbb/generated_bridge.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_test/flutter_test.dart';

class _Binding extends Fake implements RustdeskImpl {
  int installs = 0;
  @override
  String translate({required String name, required String locale, dynamic hint}) => name;
  @override
  String mainGetAppNameSync({dynamic hint}) => 'RustDesk';
  @override
  String mainGetLocalOption({required String key, dynamic hint}) => '';
  @override
  String getLocalFlutterOption({required String k, dynamic hint}) => '';
  @override
  bool isDisableAb({dynamic hint}) => false;
  @override
  bool isDisableAccount({dynamic hint}) => false;
  @override
  bool isDisableGroupPanel({dynamic hint}) => false;
  @override
  String mainGetCommonSync({required String key, dynamic hint}) =>
      '{"total_size":1,"downloaded_size":1,"finished":true}';
  @override
  Future<void> mainSetCommon({required String key, required String value, dynamic hint}) async {
    if (key == 'update-me') installs++;
  }
}

void main() {
  testWidgets('private update installs only after confirmation, never after waiting or cancel', (tester) async {
    final binding = _Binding();
    platformFFI.setBindingForTesting(binding);
    isTest = true;
    final channel = Platform.isWindows ? SystemChannels.mouseCursor : const MethodChannel('flutter_custom_cursor');
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel,
        (call) async => (call.arguments as Map<dynamic, dynamic>)['name']);
    addTearDown(() {
      gFFI.dialogManager.dismissAll();
      isTest = false;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, null);
    });
    await initGlobalFFI();
    Future<void> completeDownload() async {
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(MaterialApp(navigatorKey: globalKey, home: Scaffold(body: UpdateProgress(
          'https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/tag/v1.5.10',
          'https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/download/v1.5.10/rustdesk-1.5.10-aarch64.dmg',
          SimpleWrapper(''), SimpleWrapper<VoidCallback>(() {})))));
      await platformFFI.tryHandle({'name': 'download-new-version', 'id': 'download'});
      await tester.pump(const Duration(milliseconds: 300));
      await tester.pump();
    }
    await completeDownload();
    await tester.pump(const Duration(seconds: 6));
    expect(binding.installs, 0);
    await tester.tap(find.text('Cancel'));
    await tester.pump();
    expect(binding.installs, 0);
    await completeDownload();
    await tester.tap(find.text('OK'));
    await tester.pumpAndSettle();
    expect(binding.installs, 1);
  });
}
