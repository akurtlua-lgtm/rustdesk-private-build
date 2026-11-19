# iOS host core 编译移植（默认关闭、尚未实现共享）

新 ios-hostcore-compile-probe 只在独立分支把已有server/rendezvous/ipc模块纳入真实iOS SDK类型检查，没有新FFI、服务启动、监听、注册、用户权限或发布。首次检查预计会揭示上游把iOS排除后遗留的Android JNI、CM、camera/resolutions等阻点；失败不能当作平台适配或任何安全验证通过。

现有客户端未启用此feature，已有运行路径保持原条件；最初的5个模块编译门禁之外，当前仅新增iOS专用的JNI/CM/摄像头/唤醒锁排除、只读过滤和停止接收器分支。不能把该feature用于产品构建。只有补齐同一现有握手/密码/严格Key/E2E、只读协议和执行边界拒绝、首帧授权、可靠本地Close及必要loopback回归后，才可提供启动入口。不得为编译补成功空服务/弱化认证，不能调用Android JNI或模拟授权。

独立只读工作流仅自有codex/ios-hostcore-*，使用固定旧版iOS SDK/Xcode16.4/ARM64、同vcpkg/Rust版本、example.invalid合成配置，复用真实buffer测试与桥接前置，只有cargo check不运行host，不使用实际服务器或签名Secrets。旧版extension与新版iOS27 SCK前端另行验证；此工作流不证明extension可链接完整host或任何共享运行。


下一步的纯协议只读策略位于客户端自有base模块、由独立mobile-host-policy feature编译，不改服务器共享hbb_common。保留原登录设备名/密码/2FA/公钥消息，仅重建画质/解码协商选项并强制关闭输入、剪贴板、文件、音频、锁屏、隐私和终端选项；拒绝非普通桌面登录、OS登录、键鼠/触摸/剪贴板/文件/终端/端口/换边/设备控制或未知消息，只保留单显示0的视频反馈。现有Connection仅在iOS+编译探针feature时，on_open前将相关权限设false，on_message最前通过该策略，早于handle_login_request_without_validation；原认证实现不复制、不自动Approve。

五个同源码Mac ARM64/Rust1.81.0单元用例验证认证与画质字段保留、拒绝输入/设备控制但保留视频反馈、拒绝非桌面/OS登录，以及停止前后的订阅和拥有者退出时撤销。新增CI显式执行base目标且要求精确5通过，不能零测试绿。它不等于完整host认证loopback、CM本地关闭、系统权限或网络会话已完成；原始编译探针与此未完成状态继续区分。

默认关闭的ScopedHost基础持有真实首帧来源和Server，停止通道使用watch并在发送者消失时撤销；连接只接受与Server同一来源的停止接收器。没有监听或公开启动入口，不调用商业sync来伪造通道。移植同时排除iOS上不存在的Android JNI、桌面CM/摄像头/分辨率调用，端口转发请求明确拒绝。尚需把所有视频采集器重建绑定到同一来源，并补齐握手和消息处理期间的可取消等待；当前停止分支不能证明这些阶段已经可靠断开。不得在这些限制与认证/加密loopback通过前开放共享入口。
