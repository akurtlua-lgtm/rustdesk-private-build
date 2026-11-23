import Foundation

// Only the explicitly authorized OS capture frontend may supply owned BGRA frames.
final class NativeSharingHost: @unchecked Sendable {
    private let lock = NSLock()
    private let worker = DispatchGroup()
    private let ended: @Sendable (Int32) -> Void
    private var token: UInt64 = 0
    private var closed = false
    private var result: Int32 = 0

    init(ended: @escaping @Sendable (Int32) -> Void) { self.ended = ended }

    func submit(_ frame: CaptureFrame) -> Bool {
        lock.lock(); defer { lock.unlock() }
        guard !closed, frame.width > 0, frame.height > 0,
              frame.width <= Int(UInt32.max), frame.height <= Int(UInt32.max) else { return false }
        return frame.pixels.withUnsafeBytes { bytes in
            guard let pointer = bytes.bindMemory(to: UInt8.self).baseAddress else { return false }
            if token == 0 {
                let created = rd_mobile_host_begin(pointer, bytes.count, UInt32(frame.width),
                                                  UInt32(frame.height), frame.stride)
                guard created != 0 else { closed = true; result = -1; return false }
                token = created
                worker.enter()
                Thread.detachNewThread { [self] in
                    let outcome = rd_mobile_host_run(created)
                    rd_mobile_host_stop(created)
                    let released = rd_mobile_host_release(created)
                    lock.lock()
                    closed = true
                    result = outcome == 0 ? released : outcome
                    let error = result
                    lock.unlock()
                    ended(error)
                    worker.leave()
                }
                return true
            }
            return rd_mobile_host_frame(token, pointer, bytes.count, UInt32(frame.width),
                                        UInt32(frame.height), frame.stride) == 0
        }
    }

    func close() {
        lock.lock(); defer { lock.unlock() }
        closed = true
        if token != 0 { rd_mobile_host_stop(token) }
    }

    func waitForStop() async {
        close()
        await withCheckedContinuation { continuation in
            worker.notify(queue: .global(qos: .utility)) { continuation.resume() }
        }
    }

    var state: (phase: Int32, error: Int32) {
        lock.lock(); defer { lock.unlock() }
        if closed { return (result == 0 ? 5 : 6, result) }
        return (token == 0 ? 1 : rd_mobile_host_state(token), result)
    }
}
