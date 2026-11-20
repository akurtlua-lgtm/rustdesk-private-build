import Foundation
import ReplayKit

@available(iOS, introduced: 15.0, obsoleted: 27.0)
final class SampleHandler: BroadcastCaptureHandler, @unchecked Sendable {
    private struct Session {
        let nonce: String
        let host: NativeSharingHost
        let delivery: CaptureDelivery
        let timer: DispatchSourceTimer
    }
    private let sessionLock = NSLock()
    private var session: Session?

    override func makeDelivery() throws -> CaptureDelivery {
        let bootstrap = try SharingBootstrap.consume()
        try bootstrap.configureExtension()
        let nonce = bootstrap.nonce
        let host = NativeSharingHost { [weak self] error in
            if error != 0 { self?.fail(code: error) }
        }
        let delivery = CaptureDelivery(consumer: { host.submit($0) }, didClose: { [weak self] reason in
            host.close()
            if reason != .user { self?.fail(code: reason.rawValue) }
        })
        let timer = DispatchSource.makeTimerSource(queue: .global(qos: .utility))
        timer.schedule(deadline: .now(), repeating: .milliseconds(500))
        timer.setEventHandler { [weak self] in self?.poll() }
        sessionLock.lock()
        session = Session(nonce: nonce, host: host, delivery: delivery, timer: timer)
        sessionLock.unlock()
        timer.resume()
        return delivery
    }
    private func poll() {
        sessionLock.lock(); defer { sessionLock.unlock() }
        guard let current = session else { return }
        do {
            if try SharingBootstrap.stopRequested(current.nonce) {
                current.host.close()
                DispatchQueue.main.async { [weak self] in self?.fail(code: CaptureStopReason.user.rawValue) }
                return
            }
            let state = current.host.state
            let id = String(data: try SharingBootstrap.snapshot(rd_mobile_host_id), encoding: .utf8) ?? ""
            try SharingBootstrap.writeStatus(SharingStatus(nonce: current.nonce,
                phase: state.phase >= 4 ? 4 : state.phase, error: state.error, deviceID: id))
        } catch {
            DispatchQueue.main.async { [weak self] in self?.fail(code: CaptureStopReason.delivery.rawValue) }
        }
    }
    private func fail(code: Int32) {
        sessionLock.lock()
        let host = session?.host
        sessionLock.unlock()
        host?.close()
        DispatchQueue.main.async { [weak self] in
            self?.finishBroadcastWithError(NSError(domain: "PrivateCapture", code: Int(code)))
        }
    }
    override func broadcastFinished() {
        super.broadcastFinished()
        sessionLock.lock()
        let current = session
        session = nil
        sessionLock.unlock()
        guard let current else { return }
        current.timer.cancel()
        Task {
            await current.host.waitForStop()
            let state = current.host.state
            do {
                try SharingBootstrap.writeStatus(SharingStatus(nonce: current.nonce,
                    phase: state.error == 0 ? 5 : 6, error: state.error, deviceID: ""))
            } catch {
                NSLog("Broadcast stop receipt unavailable")
            }
        }
    }
}
