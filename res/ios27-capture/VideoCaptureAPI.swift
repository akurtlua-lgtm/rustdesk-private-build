#if targetEnvironment(simulator) || !canImport(ScreenCaptureKit)
enum VideoCaptureAPI {
    static let isAvailable = false
}
#else
import Dispatch
import ScreenCaptureKit

@available(iOS 27.0, *)
enum VideoCaptureAPI {
    static let isAvailable = true

    static func makeStream(
        filter: SCContentFilter,
        output: any SCStreamOutput,
        delegate: any SCStreamDelegate,
        queue: DispatchQueue
    ) throws -> SCStream {
        let configuration = SCStreamConfiguration()
        configuration.capturesAudio = false
        let stream = SCStream(filter: filter, configuration: configuration, delegate: delegate)
        try stream.addStreamOutput(output, type: .screen, sampleHandlerQueue: queue)
        return stream
    }

    @MainActor
    static func presentPicker(observer: any SCContentSharingPickerObserver) {
        var configuration = SCContentSharingPickerConfiguration()
        configuration.showsMicrophoneControl = false
        configuration.showsCameraControl = false
        let picker = SCContentSharingPicker.shared
        picker.defaultConfiguration = configuration
        picker.add(observer)
        picker.isActive = true
        picker.present()
    }

    @MainActor
    static func dismissPicker(observer: any SCContentSharingPickerObserver) {
        let picker = SCContentSharingPicker.shared
        picker.remove(observer)
        picker.isActive = false
    }
}
#endif
