# iOS 27 原生采集 API 开发门禁

这是独立开发分支的真实 SDK 类型检查，不是已可用的共享功能。VideoCaptureAPI 提供视频专用 stream 工厂及系统 picker API 验证，没有接入应用 target、Rust host、网络或账号，不安装/启动、不录屏、不请求权限。未开始 stream，输出仅为 SDK 检查记录，不是可安装包。

Apple 当前文档将 ScreenCaptureKit 的 iOS 支持标为27.0起，旧 RPBroadcastSampleHandler 从27.0弃用；不能把 Xcode16.4 的广播扩展探针当作 iOS27 兼容证明。iOS15–26的 ReplayKit 与 iOS27+ ScreenCaptureKit 是不同生命周期路径，后者不要求广播扩展，完整显示后台采集需要 screen-capture background mode 及系统用户确认。没有给现有应用增加这些权限或后台模式。

调用者将来必须拥有唯一采集生命周期，取消/失败/停止时撤销本地 transport、移除自身 observer 并等待系统停止完成。此工厂不替调用者实现这些行为，不能直接开放给用户；没有远控输入能力或授权授予。仅注册.screen输出且capturesAudio=false，摄像头/麦克风控制隐藏；iOS27 SDK明确禁止设置Mac专有pixelFormat/minimumFrameInterval/queueDepth/captureMicrophone，实际输出格式/跨度/颜色须由后续采集回调检查并转换，不能假定BGRA，不保存画面到文件或日志。采集缓冲/方向/布局/码率、首帧门禁、账号隔离与现有严格Key/密码/E2E host仍待集成。

新增工作流仅本仓库 codex/ios-capture-api-*，contents:read、无Secrets，使用GitHub官方xcode-27 ARM64镜像分别对device/simulator真实SDK执行Swift6 warnings-as-errors typecheck。记录真实来源/run/Xcode版本；不把类型检查当作运行/授权或真机验收。没有修改既有运行文件或桌面/手机发布门禁。

来源：
- https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-on-ios
- https://developer.apple.com/documentation/screencapturekit
- https://developer.apple.com/documentation/replaykit/rpbroadcastsamplehandler
- https://github.blog/changelog/2026-10-15-xcode-27-runner-image-generally-available/

首轮真实SDK27.1检查正确拒绝上述四个Mac专有属性；已移除不可用调用，保留双SDK Swift6 warnings-as-errors门禁，不能将此失败记录为通过。
