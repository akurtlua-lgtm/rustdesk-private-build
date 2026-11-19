# iOS 屏幕共享：扩展链接门禁

本分支先验证iOS15–26的真实 Broadcast Upload Extension 能否链接现有 Rust 核心，不启动 Flutter engine；不是可用的手机共享功能，不包含主应用入口、采集传输、App Group/共享 Keychain 或 host 认证服务。扩展收到启动请求会立即明确失败，不录屏、不开放连接或输入。

默认关闭的 `ios-host-link-probe` feature 只增加独立加密 roundtrip C 入口。已有 `src/lib.rs` 只新增该模块门禁，默认发行路径没有改动；Cargo 不增加依赖，单个单元用例检查初始化后可完成认证加密。

隔离工作流只在自己的 `codex/ios-host-*` 分支运行，固定 Xcode16.4、Rust1.81、vcpkg 基线和 ARM64；分别构建模拟器/设备 SDK 的真实 extension，开启 application-extension-only 检查。确认平台、ABI、Rust 入口、无 Flutter/Dart 或应用专属 API 依赖后仅保存 unsigned 开发 Artifact，不安装、不签名、不发布。全程只用 example.invalid 合成配置，没有服务器或长期更新/安装签名 Secrets。

下一阶段才能接有界采集缓冲、默认关闭的只读 host、现有 Key/密码/E2E 和强制本地关闭连接；旧版iOS主应用只承载系统共享 picker 与配置，host 在广播扩展进程运行；iOS27另见下方新版API说明。账号 token 不进入共享容器。人工审批没有可靠跨进程界面时应拒绝该模式；停止录屏后必须取消本地 transport，不能仅向对端发送 stop 消息。

首轮链接门禁已在 CI 44470197087 的 device/simulator 两 SDK 通过（source 2e331b74741219f19b46bffca9c1569650e69a48）；下载的两个真实 extension、来源/API摘要/内部摘要、ARM64/精确SDK/已定义C入口及日志隐私已独立核验。它不是运行测试，不能据此声称录屏授权、跨应用/后台共享或真机验收。这个独立分支不合入当前手机候选，不修改生产服务器、账号后台、权限、密钥或桌面更新流程。

## 采集帧基础模块（未接系统采集或 host）

默认关闭的 `mobile-view-only-host` 目前只编译新 `scrap::mobile_frame` 基础模块，不提供可用 host。它只接受已归一化的 packed BGRA，验证正尺寸、真实行跨度、所需长度、溢出和32MiB上限，再复制有效行；不把 NV12 planes 冒充 packed 像素。

每次采集应创建独立 `FrameInbox`，串行producer和单个consumer，pending最多一帧；返回帧的所有权交给consumer，供后续Capturer另持有。32MiB是单帧上限，不是整个进程总内存上限，close也不能撤回已经取走的帧。新模块的状态只由 publish、dimensions、take_frame、close 读取：publish在同一锁内更新尺寸和pending，dimensions供未来Display枚举，take_frame要求与编码器尺寸一致（不一致时保留给重建后的capturer），close清帧和尺寸且该实例永不重开。没有过期计时、帧队列或旧采集路径改写。原生SDK回调、颜色转换、首帧授权门禁和本地断连尚未接入，关闭缓冲不等于已停止系统录屏。

三个同源码原生 ARM64 Rust1.81.0 测试实际通过：带padding与非法布局、只读取最新帧且不送给错误尺寸编码器、关闭后拒绝收发。新工作流显式执行此纯std模块测试；Cargo默认workspace测试也能发现该测试模块，不要求额外原生mock层。完整SDK/采集/编码/协议链路仍未验收。

下一轮仅为 iOS+feature 提供借用已校验 BGRA 的 concrete PixelBuffer，并放开现有 Frame/convert_to_yuv 的编译门禁；不开放 camera、Display/Capturer、server 或修改 trait 签名。新增独立 required-features Cargo 集成测试目标，经 simctl spawn 在真实匹配 SDK 的 iOS 模拟器进程执行合成色块→Frame.to→VP8生产packet→认证Box→解码，核对尺寸/通道与颜色容差。像素仅为常量，Box密钥仅内存生成，不建立网络或绕过系统录屏授权。不能把这个 CPU 测试当作 Broadcast Extension、录屏授权、真实手机共享或远控会话已经运行。

## 默认关闭的 iOS 合成采集后端（系统采集与 host 仍未接入）

新 mobile_capture 只将现有 FrameInbox 适配为既有签名的 Display/Capturer，不修改其他平台 trait 签名。iOS+feature 下恢复原本缺失的 frame 方法；feature关闭时保持原条件。begin_capture 需要至少一个有效帧，并只允许一个活跃 lease；没有收到首帧或已关闭时不枚举显示。Display保存源与尺寸快照，Capturer要求尺寸一致，frame只从该源读取匹配尺寸的最新帧；旋转或缩小均明确失败交给将来的显示服务重建，不能送错编码器。

状态只在此新模块：SOURCE弱引用注册表由begin_capture/lease Drop/Display::all读取；lease持有源，Drop先关闭它，再只清除此实例的注册项。Display::all/primary/is_online、新Capturer与frame读取同一源；旧Display/Capturer永远保留旧实例，不跟随新注册。SOURCE与FrameInbox没有跨await或同时反向持锁，关闭后新显示不可见、既有capturer不能再取帧。仍不能撤回已取出的帧，也不等于OS录屏停止或连接已经本地断开；尚无外部FFI、真实采集或host调用此模块。

新目标运行测试增加一个公开行为用例：首帧前不可用、重复注册拒绝、尺寸变化拒绝旧capturer、匹配新尺寸后可取帧、lease结束后既有capturer报关闭且不能再发布。模拟器脚本现在要求原有CPU媒体用例与此用例共精确2项通过；实际SDK/运行结果待下一候选，不能把新代码或仅类型检查标为通过。

2026-11-19重新核对Apple官方文档：上述Broadcast Extension探针只针对iOS15–26旧路径。iOS27起ScreenCaptureKit支持iOS、RPBroadcastSampleHandler已弃用，新系统改为主应用的系统picker/SCStream及screen-capture后台模式，单独开发SDK兼容验证见codex/ios-capture-api-*。两种OS路径的采集生命周期不同，都仍需同一认证host/首帧授权/强制本地断连，不能用新的后台模式绕过用户确认。https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-on-ios

第三轮run44479396314双SDK扩展链接通过，但模拟器CPU测试命令在20分钟编译截止时仍有rustc进程，被正确标记失败，没有测试通过记录。下一轮先cargo test --no-run在独立编译预算内完成，再用相同配置实际执行两项测试、180秒运行截止；不移除任何断言、feature或真实目标运行，编译/运行失败均输出仅有界脱敏诊断，不能以编译完成冒充运行通过。

最小化复查发现CPU/缓冲测试只使用scrap、base与hbb_common，不使用根应用。Cargo官方说明根包integration test会顺带构建该包所有binary；根测试目标因此会带入无关桌面程序。现移除本PR先前新增的根[[test]]，把同源码/断言移到scrap/tests并用-p scrap执行，不新建测试包或依赖。scrap旧dev-dependencies中的桌面quest/docopt/repng仅对原有Android之外再排除iOS；正常生产依赖不变，其他平台测试声明保持，避免新iOS测试引入桌面GL工具。Root真正双SDK扩展链接仍照常执行，不用采集库测试冒充Root host运行。https://doc.rust-lang.org/cargo/commands/cargo-test.html
