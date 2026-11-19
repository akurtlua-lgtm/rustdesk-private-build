# iOS 屏幕共享：扩展链接门禁

本分支先验证真实 Broadcast Upload Extension 能否链接现有 Rust 核心，不启动 Flutter engine；不是可用的手机共享功能，不包含主应用入口、采集传输、App Group/共享 Keychain 或 host 认证服务。扩展收到启动请求会立即明确失败，不录屏、不开放连接或输入。

默认关闭的 `ios-host-link-probe` feature 只增加独立加密 roundtrip C 入口。已有 `src/lib.rs` 只新增该模块门禁，默认发行路径没有改动；Cargo 不增加依赖，单个单元用例检查初始化后可完成认证加密。

隔离工作流只在自己的 `codex/ios-host-*` 分支运行，固定 Xcode16.4、Rust1.81、vcpkg 基线和 ARM64；分别构建模拟器/设备 SDK 的真实 extension，开启 application-extension-only 检查。确认平台、ABI、Rust 入口、无 Flutter/Dart 或应用专属 API 依赖后仅保存 unsigned 开发 Artifact，不安装、不签名、不发布。全程只用 example.invalid 合成配置，没有服务器或长期更新/安装签名 Secrets。

下一阶段才能接有界采集缓冲、默认关闭的只读 host、现有 Key/密码/E2E 和强制本地关闭连接；主应用只承载系统共享 picker 与配置，host 在广播扩展进程运行。账号 token 不进入共享容器。人工审批没有可靠跨进程界面时应拒绝该模式；停止录屏后必须取消本地 transport，不能仅向对端发送 stop 消息。

首轮链接门禁已在 CI 44470197087 的 device/simulator 两 SDK 通过（source 2e331b74741219f19b46bffca9c1569650e69a48）；下载的两个真实 extension、来源/API摘要/内部摘要、ARM64/精确SDK/已定义C入口及日志隐私已独立核验。它不是运行测试，不能据此声称录屏授权、跨应用/后台共享或真机验收。这个独立分支不合入当前手机候选，不修改生产服务器、账号后台、权限、密钥或桌面更新流程。

## 采集帧基础模块（未接系统采集或 host）

默认关闭的 `mobile-view-only-host` 目前只编译新 `scrap::mobile_frame` 基础模块，不提供可用 host。它只接受已归一化的 packed BGRA，验证正尺寸、真实行跨度、所需长度、溢出和32MiB上限，再复制有效行；不把 NV12 planes 冒充 packed 像素。

每次采集应创建独立 `FrameInbox`，串行producer和单个consumer，pending最多一帧；返回帧的所有权交给consumer，供后续Capturer另持有。32MiB是单帧上限，不是整个进程总内存上限，close也不能撤回已经取走的帧。新模块的状态只由 publish、dimensions、take_frame、close 读取：publish在同一锁内更新尺寸和pending，dimensions供未来Display枚举，take_frame要求与编码器尺寸一致（不一致时保留给重建后的capturer），close清帧和尺寸且该实例永不重开。没有过期计时、帧队列或旧采集路径改写。原生SDK回调、颜色转换、首帧授权门禁和本地断连尚未接入，关闭缓冲不等于已停止系统录屏。

三个同源码原生 ARM64 Rust1.81.0 测试实际通过：带padding与非法布局、只读取最新帧且不送给错误尺寸编码器、关闭后拒绝收发。新工作流显式执行此纯std模块测试；Cargo默认workspace测试也能发现该测试模块，不要求额外原生mock层。完整SDK/采集/编码/协议链路仍未验收。

下一轮仅为 iOS+feature 提供借用已校验 BGRA 的 concrete PixelBuffer，并放开现有 Frame/convert_to_yuv 的编译门禁；不开放 camera、Display/Capturer、server 或修改 trait 签名。新增独立 required-features Cargo 集成测试目标，经 simctl spawn 在真实匹配 SDK 的 iOS 模拟器进程执行合成色块→Frame.to→VP8生产packet→认证Box→解码，核对尺寸/通道与颜色容差。像素仅为常量，Box密钥仅内存生成，不建立网络或绕过系统录屏授权。不能把这个 CPU 测试当作 Broadcast Extension、录屏授权、真实手机共享或远控会话已经运行。


### 实际图像缓冲转换候选

新增独立 CaptureFrame 转换器，用 CoreImage 读取 CVPixelBuffer 的真实格式和图像元数据，仅接受 BGRA 及8位双平面 NV12，按明确方向生成自有连续 BGRA 数据；拒绝不支持格式、无效尺寸、超过32MiB输出或渲染失败。输出供后续同一受授权 Scope 发布，不启动采集或网络，不保留系统缓冲裸指针。两项合成 CoreVideo 原生测试覆盖带行padding的红/蓝图90度旋转及NV12视频范围黑色转换；本机原生ARM64 Swift6实际编译和运行通过，不能代替iOS验证。工作流新增真实设备SDK类型检查和iOS27模拟器编译/执行同两项测试，只有返回码及精确成功标记通过才上传来源绑定证明。仍没有系统采集回调接入或公开手机共享入口，也没有真机验收。
