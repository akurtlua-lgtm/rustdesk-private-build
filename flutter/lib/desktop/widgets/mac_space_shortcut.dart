Future<void> sendMacSpaceShortcut({
  required bool right,
  required bool swapCtrlCmd,
  required Future<void> Function(
          String name, bool press, bool ctrl, bool command)
      sendKey,
}) async {
  try {
    // Session input swaps these modifiers again before sending to the peer.
    await sendKey(
        right ? 'VK_RIGHT' : 'VK_LEFT', true, !swapCtrlCmd, swapCtrlCmd);
  } finally {
    // Legacy key presses leave their modifier held until explicitly released.
    await sendKey(swapCtrlCmd ? 'Meta' : 'VK_CONTROL', false, false, false);
  }
}
