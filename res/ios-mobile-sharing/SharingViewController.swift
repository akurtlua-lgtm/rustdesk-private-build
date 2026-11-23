import Foundation
import ReplayKit
import UIKit

@MainActor
final class SharingViewController: UIViewController {
    private let statusLabel = UILabel()
    private let idLabel = UILabel()
    private let password = UITextField()
    private let startButton = UIButton(type: .system)
    private let stopButton = UIButton(type: .system)
    private let picker = UIStackView()
    private var host: NativeSharingHost?
    private var capture: AnyObject?
    private var legacyNonce: String?
    private var timer: Timer?
    private var stopping = false

    override func viewDidLoad() {
        super.viewDidLoad()
        isModalInPresentation = true
        view.backgroundColor = .systemBackground
        let title = UILabel()
        title.text = "共享本机屏幕"
        title.font = .preferredFont(forTextStyle: .title2)
        statusLabel.accessibilityIdentifier = "private-sharing-status"
        startButton.accessibilityIdentifier = "private-sharing-start"
        statusLabel.numberOfLines = 0
        statusLabel.text = "每次都需要系统授权和设备密码；仅供查看。"
        password.placeholder = "设置设备密码（至少 8 个字符）"
        password.isSecureTextEntry = true
        password.textContentType = .newPassword
        password.borderStyle = .roundedRect
        startButton.setTitle("确认并打开系统共享提示", for: .normal)
        startButton.addTarget(self, action: #selector(startSharing), for: .touchUpInside)
        stopButton.setTitle("停止共享", for: .normal)
        stopButton.addTarget(self, action: #selector(stopTapped), for: .touchUpInside)
        let close = UIButton(type: .system)
        close.setTitle("关闭", for: .normal)
        close.addTarget(self, action: #selector(closeTapped), for: .touchUpInside)
        let detail = UILabel()
        detail.numberOfLines = 0
        detail.text = "不共享麦克风、声音或摄像头，不接受键鼠或文件操作。账号登录不会代替设备授权。"
        detail.font = .preferredFont(forTextStyle: .footnote)
        if #unavailable(iOS 27.0) {
            let broadcast = RPSystemBroadcastPickerView(frame: .zero)
            broadcast.showsMicrophoneButton = false
            broadcast.preferredExtension = (Bundle.main.bundleIdentifier ?? "") + ".BroadcastShare"
            picker.addArrangedSubview(broadcast)
        }
        picker.isHidden = true
        let stack = UIStackView(arrangedSubviews: [title, statusLabel, idLabel, password,
            startButton, picker, stopButton, detail, close])
        stack.axis = .vertical
        stack.spacing = 16
        stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: view.safeAreaLayoutGuide.leadingAnchor, constant: 24),
            stack.trailingAnchor.constraint(equalTo: view.safeAreaLayoutGuide.trailingAnchor, constant: -24),
            stack.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor, constant: 24),
            picker.heightAnchor.constraint(equalToConstant: 60)
        ])
        #if targetEnvironment(simulator)
        startButton.isEnabled = false
        statusLabel.text = "模拟器不支持系统屏幕共享；当前仅验证界面、采集转换和加密连接核心。"
        #endif
        #if !targetEnvironment(simulator)
        if #unavailable(iOS 27.0) {
            do {
                if let existing = try SharingBootstrap.status(), existing.phase < 5 {
                    legacyNonce = existing.nonce
                    startButton.isEnabled = false
                    statusLabel.text = "检测到尚未确认停止的广播，请先停止原共享。"
                }
            } catch { statusLabel.text = "无法读取共享扩展状态，请检查应用签名与共享组配置。" }
        }
        #endif
        timer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            Task { @MainActor in self?.refresh() }
        }
    }
    deinit { timer?.invalidate() }

    @objc private func startSharing() {
        guard host == nil, capture == nil, legacyNonce == nil, !stopping else { return }
        let value = password.text ?? ""
        password.text = nil
        let bytes = Array(value.utf8)
        guard bytes.withUnsafeBufferPointer({ rd_mobile_host_set_password($0.baseAddress, $0.count) }) == 0 else {
            statusLabel.text = "无法设置设备密码，请使用至少 8 个字符并检查安全设置。"; return
        }
        do {
            #if canImport(ScreenCaptureKit) && !targetEnvironment(simulator)
            if #available(iOS 27.0, *) {
                let created = NativeSharingHost { [weak self] _ in
                    Task { @MainActor in await self?.stopSharing() }
                }
                host = created
                let delivery = CaptureDelivery(consumer: { created.submit($0) }, didClose: { [weak self] _ in
                    created.close()
                    Task { @MainActor in await self?.stopSharing() }
                })
                let frontend = SharingCapture(delivery: delivery)
                capture = frontend
                try frontend.present()
                statusLabel.text = "请在系统提示中确认共享屏幕"
                startButton.isEnabled = false
                return
            }
            #endif
            guard #unavailable(iOS 27.0) else { throw SharingBootstrap.Failure.configuration }
            let prepared = try SharingBootstrap.prepare(password: value)
            legacyNonce = prepared.nonce
            picker.isHidden = false
            startButton.isEnabled = false
            statusLabel.text = "请点击下方系统广播按钮并确认共享；共享密码与双重验证仍生效。"
        } catch {
            statusLabel.text = "无法准备共享，请检查应用签名及共享扩展配置。"
            if host != nil { Task { await stopSharing() } }
        }
    }
    private func refresh() {
        guard !stopping else { return }
        if let host {
            let state = host.state
            if state.phase == 2 { statusLabel.text = "已获系统授权，正在注册加密共享服务" }
            if state.phase == 3 { statusLabel.text = "共享已就绪，等待远端凭设备密码连接（仅查看）" }
            if state.phase >= 4 { Task { await stopSharing() } }
            do { idLabel.text = "本机 ID：" + (String(data: try SharingBootstrap.snapshot(rd_mobile_host_id), encoding: .utf8) ?? "") }
            catch { statusLabel.text = "无法读取本机共享状态" }
        }
        if let nonce = legacyNonce {
            do {
                guard let state = try SharingBootstrap.status(), state.nonce == nonce else { return }
                idLabel.text = state.deviceID.isEmpty ? "" : "共享设备 ID：" + state.deviceID
                if state.phase == 2 { statusLabel.text = "系统广播已授权，正在注册加密共享服务" }
                if state.phase == 3 { statusLabel.text = "共享已就绪，等待远端凭设备密码连接（仅查看）" }
                if state.phase >= 5 {
                    legacyNonce = nil
                    picker.isHidden = true
                    startButton.isEnabled = true
                    statusLabel.text = state.error == 0 ? "共享已停止" : "共享已停止，核心报告错误"
                    try SharingBootstrap.discard()
                }
            } catch { statusLabel.text = "无法读取共享扩展状态，请检查系统共享指示器。" }
        }
    }
    @objc private func stopTapped() { Task { await stopSharing() } }
    @objc private func closeTapped() {
        Task {
            await stopSharing()
            if host == nil && capture == nil && legacyNonce == nil {
                timer?.invalidate(); timer = nil
                dismiss(animated: true)
            }
        }
    }
    private func stopSharing() async {
        guard !stopping else { return }
        stopping = true
        defer { stopping = false }
        host?.close()
        var captureStopped = true
        #if canImport(ScreenCaptureKit) && !targetEnvironment(simulator)
        if #available(iOS 27.0, *), let frontend = capture as? SharingCapture {
            do { try await frontend.stop(); capture = nil }
            catch { captureStopped = false }
        }
        #endif
        if let host { await host.waitForStop() }
        host = nil
        if let nonce = legacyNonce {
            do {
                try SharingBootstrap.requestStop(nonce)
                try SharingBootstrap.discard()
                for _ in 0..<25 {
                    if let state = try SharingBootstrap.status(), state.nonce == nonce, state.phase >= 5 {
                        legacyNonce = nil; break
                    }
                    try await Task.sleep(nanoseconds: 400_000_000)
                }
            } catch { statusLabel.text = "停止请求未完成，请检查系统共享指示器。"; return }
        }
        let stopped = captureStopped && legacyNonce == nil
        statusLabel.text = stopped ? "共享已停止" : "停止请求已发送；尚未确认系统停止，请停止系统共享后重试。"
        startButton.isEnabled = stopped
        picker.isHidden = true
    }
}
