import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/desktop/widgets/mac_space_shortcut.dart';

void main() {
  test('both directions reach macOS as Control-arrow with or without swap',
      () async {
    for (final right in [false, true]) {
      for (final swapped in [false, true]) {
        final events = <List<Object>>[];
        await sendMacSpaceShortcut(
          right: right,
          swapCtrlCmd: swapped,
          sendKey: (name, press, ctrl, command) async {
            events.add([name, press, ctrl, command]);
          },
        );
        final press = events.first;
        expect(press[0], right ? 'VK_RIGHT' : 'VK_LEFT');
        expect(press[1], isTrue);
        // Emulate the existing session's Ctrl/Command swap at the boundary.
        expect(swapped ? press[3] : press[2], isTrue);
        expect(swapped ? press[2] : press[3], isFalse);
        expect(events.last,
            [swapped ? 'Meta' : 'VK_CONTROL', false, false, false]);
        expect(events.length, 2);
      }
    }
  });

  test('a failed press still releases its modifier and propagates the error',
      () async {
    final events = <String>[];
    await expectLater(
      sendMacSpaceShortcut(
        right: true,
        swapCtrlCmd: false,
        sendKey: (name, press, ctrl, command) async {
          events.add(name);
          if (press) throw StateError('input unavailable');
        },
      ),
      throwsStateError,
    );
    expect(events, ['VK_RIGHT', 'VK_CONTROL']);
  });
}
