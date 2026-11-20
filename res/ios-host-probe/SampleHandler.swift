import Foundation
import ReplayKit

@_silgen_name("rd_ios_host_link_probe")
private func cryptoLinkProbe() -> Int32

class SampleHandler: RPBroadcastSampleHandler {
    override func broadcastStarted(withSetupInfo setupInfo: [String: NSObject]?) {
        let cryptoReady = cryptoLinkProbe() == 1
        finishBroadcastWithError(NSError(
            domain: "io.github.akurtlualgtm.rustdesk.broadcast-probe",
            code: cryptoReady ? 1 : 2,
            userInfo: [NSLocalizedDescriptionKey: "Development link probe only; screen sharing is unavailable."]
        ))
    }
}
