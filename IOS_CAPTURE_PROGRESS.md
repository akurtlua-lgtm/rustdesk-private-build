# iOS 系统采集前端接入候选

默认不进入现有主控应用或正式安装包，不修改服务端、账号、设备授权和加密策略。应用须提供受授权的同步画面消费者，完成现有 Key、密码/TOTP、只读协议及来源关闭门禁；没有可用消费者时不能显示共享可用。

`CaptureDelivery` 在系统授权后才转换和交付真实 CVPixelBuffer，自有 packed BGRA 通过同步消费者送往有界来源；回调不持有借用裸指针，不排队创建每帧 Task。取消、错误和关闭使该实例永久拒绝新帧，关闭等待当前同步消费者完成；消费者拒绝或转换失败也会关闭。消费者和关闭回调必须非重入，不得反向同步调用同一个 Delivery。

iOS27 的 `SharingCapture` 经系统内容选择器创建和启动 ScreenCaptureKit stream；明确关闭音频，队列深度3、目标30FPS、BGRA。只有 startCapture 成功后才授权画面交付；停止先撤销交付，再等待在途启动结束并停止系统流。当前 Scope 不支持更换内容源：系统选择器为当前 stream 返回新 filter 时会立即撤销交付并停止流，须重新开始，而不会继续传旧内容。异步系统停止失败保留 `systemStopError` 和流供调用者明确处置，不把交付关闭误报为系统已停止。模拟器仍明确不支持 SCK，不能用现有主控模拟器启动测试冒充录屏。

iOS15–26 的 `BroadcastCaptureHandler` 是 ReplayKit 广播适配基类。具体应用子类必须提供真实的 scoped host 消费者，默认实现明确报不支持；系统 broadcastStarted 后授权画面，按 sample 的方向元数据转换 BGRA/NV12，仅处理视频。结束和暂停撤销当前共享；暂停后需要重新开始，不假装可以恢复已撤销的来源。它没有把独立扩展进程中的对象当作主应用对象共享，也不传递 HTTPS 账号 token。

真实 SDK 门禁分别检查旧 SDK18 的 device/simulator ReplayKit 前端和 SDK27 的 device SCK（simulator明确不支持），并在当前 iOS 模拟器运行两项 CoreVideo 转换及一项授权交付/关闭行为测试。构建/运行证据以候选源码 CI 为准，不把类型检查、模拟帧或现有主控安装视为跨应用采集和后台授权通过。

回归面：现有 `CaptureFrame` 只增加其不可变自有数据的 Sendable 声明；现有 VideoCaptureAPI 仅供独立新采集候选使用，新增明确的像素格式/队列及picker移除；现有合成测试保留两项转换并追加一项行为断言。独立工作流新增旧 SDK 门禁，现有运行器保持退出码、精确成功标记、源码及模拟器身份校验，只把真实用例数从2同步为3。主控、桌面和已发布 Android 路径不变。
