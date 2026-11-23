import Flutter
import UIKit

@MainActor
enum SharingPlugin {
    static func register(controller: FlutterViewController) {
        let channel = FlutterMethodChannel(name: "private.rustdesk/mobile-sharing",
                                            binaryMessenger: controller.binaryMessenger)
        channel.setMethodCallHandler { [weak controller] call, result in
            guard call.method == "present" else { result(FlutterMethodNotImplemented); return }
            guard let controller, controller.presentedViewController == nil else {
                result(FlutterError(code: "busy", message: "请先关闭当前窗口", details: nil)); return
            }
            controller.present(SharingViewController(), animated: true)
            result(nil)
        }
    }
}
