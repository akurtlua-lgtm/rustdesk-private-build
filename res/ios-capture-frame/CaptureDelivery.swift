import CoreImage
import CoreVideo
import Foundation
import ImageIO

enum CaptureStopReason: Int32, Sendable {
    case user = 1, system, conversion, delivery, startup, unsupported, paused
}

// Consumers are synchronous, non-reentrant and may retain only the owned CaptureFrame.
final class CaptureDelivery: @unchecked Sendable {
    private let lock = NSLock()
    private let context: CIContext
    private let consumer: @Sendable (CaptureFrame) -> Bool
    private let didClose: @Sendable (CaptureStopReason) -> Void
    private var authorized = false
    private var closed = false

    init(context: CIContext = CIContext(),
         consumer: @escaping @Sendable (CaptureFrame) -> Bool,
         didClose: @escaping @Sendable (CaptureStopReason) -> Void) {
        self.context = context
        self.consumer = consumer
        self.didClose = didClose
    }

    var isClosed: Bool {
        lock.lock(); defer { lock.unlock() }
        return closed
    }

    func authorize() {
        lock.lock(); defer { lock.unlock() }
        if !closed { authorized = true }
    }

    func submit(_ buffer: CVPixelBuffer, orientation: CGImagePropertyOrientation) {
        lock.lock(); defer { lock.unlock() }
        guard authorized, !closed else { return }
        do {
            let frame = try CaptureFrame.convert(buffer, orientation: orientation, context: context)
            if !consumer(frame) { closeLocked(.delivery) }
        } catch {
            closeLocked(.conversion)
        }
    }

    func close(_ reason: CaptureStopReason) {
        lock.lock(); defer { lock.unlock() }
        closeLocked(reason)
    }

    private func closeLocked(_ reason: CaptureStopReason) {
        guard !closed else { return }
        closed = true
        authorized = false
        didClose(reason)
    }
}
