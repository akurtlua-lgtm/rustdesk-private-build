#if canImport(ScreenCaptureKit) && !targetEnvironment(simulator)
import CoreMedia
import Foundation
import ScreenCaptureKit

@available(iOS 27.0, *)
@MainActor
final class SharingCapture: NSObject, @MainActor SCContentSharingPickerObserver, SCStreamOutput, SCStreamDelegate {
    nonisolated let delivery: CaptureDelivery
    private let queue = DispatchQueue(label: "private-capture-video")
    private var stream: SCStream?
    private var starting: Task<Void, Never>?
    private var picking = false
    private(set) var systemStopError: (any Error)?

    init(delivery: CaptureDelivery) {
        self.delivery = delivery
    }

    func present() throws {
        guard !picking, stream == nil, !delivery.isClosed else {
            throw NSError(domain: "PrivateCapture", code: Int(CaptureStopReason.startup.rawValue))
        }
        picking = true
        VideoCaptureAPI.presentPicker(observer: self)
    }

    func stop() async throws {
        delivery.close(.user)
        picking = false
        VideoCaptureAPI.dismissPicker(observer: self)
        if let starting {
            starting.cancel()
            await starting.value
            self.starting = nil
        }
        if let stream {
            try await stream.stopCapture()
            self.stream = nil
        }
        systemStopError = nil
    }

    func contentSharingPicker(_ picker: SCContentSharingPicker,
                              didUpdateWith filter: SCContentFilter, for selected: SCStream?) {
        if let selected, selected === stream, !delivery.isClosed {
            delivery.close(.user)
            VideoCaptureAPI.dismissPicker(observer: self)
            Task { [self] in
                do { try await stop() }
                catch { systemStopError = error }
            }
            return
        }
        guard picking, stream == nil, selected == nil, !delivery.isClosed else { return }
        picking = false
        starting = Task { [self] in
            guard !Task.isCancelled, !delivery.isClosed else { return }
            do {
                let created = try VideoCaptureAPI.makeStream(filter: filter, output: self,
                                                             delegate: self, queue: queue)
                stream = created
                try await created.startCapture()
                if Task.isCancelled || delivery.isClosed {
                    try await created.stopCapture()
                    stream = nil
                } else {
                    delivery.authorize()
                }
            } catch {
                systemStopError = error
                VideoCaptureAPI.dismissPicker(observer: self)
                delivery.close(.startup)
            }
        }
    }

    func contentSharingPicker(_ picker: SCContentSharingPicker, didCancelFor selected: SCStream?) {
        guard picking, selected == nil else { return }
        picking = false
        VideoCaptureAPI.dismissPicker(observer: self)
        delivery.close(.user)
    }

    nonisolated func contentSharingPickerStartDidFailWithError(_ error: any Error) {
        delivery.close(.startup)
        Task { @MainActor [weak self] in
            guard let self else { return }
            self.picking = false
            VideoCaptureAPI.dismissPicker(observer: self)
        }
    }

    nonisolated func stream(_ stream: SCStream, didOutputSampleBuffer sample: CMSampleBuffer,
                             of type: SCStreamOutputType) {
        guard type == .screen, CMSampleBufferIsValid(sample), CMSampleBufferDataIsReady(sample),
              let buffer = CMSampleBufferGetImageBuffer(sample) else { return }
        delivery.submit(buffer, orientation: .up)
    }

    nonisolated func stream(_ stream: SCStream, didStopWithError error: any Error) {
        delivery.close(.system)
        Task { @MainActor [weak self] in
            guard let self else { return }
            if self.stream === stream { self.stream = nil }
            self.picking = false
            VideoCaptureAPI.dismissPicker(observer: self)
        }
    }
}
#endif
