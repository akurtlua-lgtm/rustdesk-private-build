# 原生鸿蒙手机共享接入候选

本候选现在将系统采集、Rust 共享核心、NAPI 协调器及原生 ArkTS 共享面板接入独立开发 HAP。源码默认关闭，开发工作流显式启用；没有提升正式版。现有主控、账号资料隔离、只读/键盘权限门禁保留。

新增 `Capture` 负责公开 API20 的 Create/Init、系统用户确认式 Start、Stop 和 Release；关闭麦克风、内录及摄像头。只有系统 STARTED 回调授权后，`Delivery` 才会调用同步画面消费者；取消、系统停止/中断、私密场景和错误均使它永久关闭。SDK 的 NativeBuffer 按真实映射平面、步幅与 RGBA/BGRA 格式复制为自有 BGRA，不跨回调保留缓冲指针。

SDK 回调使用不复用的整数标识查询共享存活状态，不解引用已释放的实例或上下文指针。Stop 先关闭帧交付并等待已进入的同步画面消费者返回，然后请求系统停止和资源释放；资源释放成功后才移除实例占用，失败时拒绝创建新采集。同步 Start 失败仍会尝试 Release，Stop 失败也不会跳过资源释放尝试，原始错误会返回。画面交付已经关闭不能被称作系统录屏一定停止。应用必须串行调用 create/start/stop，消费者不能重入采集对象、抛出异常或保留借用指针。若 OS 释放持续失败而应用已丢弃采集对象，该进程将保持关闭/占用并报错，需要本地系统确认停止和应用重启，不添加隐式恢复或绕过权限。

本地三项真实 ASan/UBSan 交付测试覆盖授权前拒绝画面、停止与在途消费者同步且之后拒绝画面、消费者拒绝后永久关闭；此前三项颜色/步幅/容量测试保留。本地公开 API20 ARM64 SDK 已实际编译并完成无未定义符号链接；CI 要独立执行六项行为测试并检查生命周期及 NativeBuffer 真实符号。它们是合成输入/目标编译，不是系统录屏、商业鸿蒙或真机验证。

回归面：现有 `res/ohos-capture/CMakeLists.txt` 仅在 `RD_BUILD_OHOS_CAPTURE_PROBE=ON` 时加入新文件及必要的 SDK 库；原有开关关闭行为不变。现有独立工作流增加交付行为测试和实际 SDK 符号门禁，明确记录有 Start 实现但无系统执行或 host 引擎连接。未改动 HAP manifest、主控 UI、Rust core、共享子模块或任何生产服务。

## 原生共享核心与应用入口

新增 `ohos-hostcore-compile-probe` 默认关闭特性选择已审查的手机 Scope、来源绑定采集、OSS UDP 注册/TCP 中继及原有 Connection。首次系统授权 BGRA 到达后才建立 Scope，强制 licence_key、端到端握手、设备永久密码、既有 TOTP 与协议级只读过滤。不会把 HTTPS 账号或设备资料当成远控 ACL，不启用桌面 IPC、音频、键鼠或文件服务。

NAPI 先启动仅等待帧或撤销的独立线程；首次有效帧唤醒后，仅该明确原生进程入口创建用户批准的单一运行环境。停止立即撤销 Root Scope，然后后台等待网络收尾、线程结束及 SDK Stop/Release。系统取消、转换失败与 Root 退出会关闭来源；没有用 UI 阻塞等待或回调内创建运行环境。密码临时缓冲在全部退出路径擦除。

应用在系统授权后申请公开 API20 的录屏后台任务 `audioRecording`（并不打开麦克风），仅声明 INTERNET 与 KEEP_BACKGROUND_RUNNING；申请失败则显示仅限前台并在退后台时停止。停止失败会保留可重试入口并提示清理未完成。显示“就绪”只代表注册完成等待密码认证，不代表已有远端连接。

本机已完成真实 `aarch64-unknown-linux-ohos` 的特性开启检查、release 链接和关闭检查；API20 构建得到包含实际 ArkTS、NAPI、25 个原生核心接口的 unsigned HAP，ELF/依赖/权限校验通过。Python 12 项测试通过，采集组件六项 ASan/UBSan 测试及 SDK 生命周期符号在独立 CI 已通过。当前集成版本仍需同源码 CI 复验。没有执行鸿蒙 OS 采集授权、后台系统任务或真机远程连接；SDK 为公开 API20/6.0.0.47 Beta1，不把它称为商业鸿蒙真机验证。

必要回归面：`src/lib.rs`、`src/server.rs`、server connection/display/video、`src/ipc.rs`、`src/rendezvous_mediator.rs`、`src/ui_cm_interface.rs` 及 scrap 模块条件增加 OHOS 独立入口，避免走 Linux 桌面采集/IPC；非 OHOS 原条件保持等价。音频新增明确不支持的 OHOS stub，未实例化服务。现有 Index/EntryAbility 仅增加特性门禁 UI 与后台撤销钩子。构建脚本显式选择特性/权限；产物验证默认仍为原控制端，启用时额外核对完整 host ABI 和精确权限。共享子模块固定 fae47e0，仅使用既有三文件 OHOS 补丁，不提交本地子模块修改。
