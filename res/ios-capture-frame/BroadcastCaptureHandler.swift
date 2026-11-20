import CoreMedia
import Foundation
import ImageIO
import ReplayKit

// The application supplies its scoped, credential-checked host; there is no permissive default.
@available(iOS, introduced: 15.0, obsoleted: 27.0)
class BroadcastCaptureHandler: RPBroadcastSampleHandler {
    private let lock = NSLock()
    private var delivery: CaptureDelivery?

    func makeDelivery() throws -> CaptureDelivery {
        throw NSError(domain: "PrivateCapture", code: Int(CaptureStopReason.unsupported.rawValue))
    }

    override func broadcastStarted(withSetupInfo setupInfo: [String: NSObject]?) {
        do {
            let created = try makeDelivery()
            lock.lock()
            delivery = created
            lock.unlock()
            created.authorize()
        } catch {
            finishBroadcastWithError(error)
        }
    }

    override func broadcastFinished() {
        lock.lock()
        let current = delivery
        delivery = nil
        lock.unlock()
        current?.close(.user)
    }

    override func broadcastPaused() {
        lock.lock()
        let current = delivery
        delivery = nil
        lock.unlock()
        current?.close(.paused)
        finishBroadcastWithError(NSError(domain: "PrivateCapture", code: Int(CaptureStopReason.paused.rawValue)))
    }

    override func processSampleBuffer(_ sampleBuffer: CMSampleBuffer, with type: RPSampleBufferType) {
        guard #unavailable(iOS 27.0) else { return }
        guard type == .video, CMSampleBufferIsValid(sampleBuffer), CMSampleBufferDataIsReady(sampleBuffer),
              let buffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }
        lock.lock()
        let current = delivery
        lock.unlock()
        guard let current else { return }
        let number = CMGetAttachment(sampleBuffer, key: RPVideoSampleOrientationKey as CFString,
                                     attachmentModeOut: nil) as? NSNumber
        let orientation = number.flatMap { CGImagePropertyOrientation(rawValue: $0.uint32Value) } ?? .up
        current.submit(buffer, orientation: orientation)
        if current.isClosed {
            finishBroadcastWithError(NSError(domain: "PrivateCapture", code: Int(CaptureStopReason.delivery.rawValue)))
        }
    }
}
