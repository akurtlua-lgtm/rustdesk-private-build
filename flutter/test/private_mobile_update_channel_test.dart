import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/mobile/private_build.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('private phone does not request or schedule desktop updates',
      (tester) async {
    final previousMobile = isMobile;
    isMobile = true;
    addTearDown(() => isMobile = previousMobile);
    checkUpdate();
    await tester.pump(const Duration(seconds: 2));
  }, skip: !privateMobileManualUpdates);
}
