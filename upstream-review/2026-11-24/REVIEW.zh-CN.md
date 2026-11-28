# 2026-11-24 增量范围审查：保持正式版

这是范围分诊及依赖核对，不是原生兼容性验收。继续使用同一个草稿 PR #16，不合入 PR #3 或移动开发 PR #17，不触发重复构建。

目标 main 仍为 `5479b701e2b3ff3c1485a6a641373e8f7abfb3bd`，正式桌面版 `v1.5.14+82`，上游正式版仍为 `1.5.3`。连续已集成基线仍为 `d46c7c52521a311d3ada8eeacd4f13b94e321258`。d40e1da 已随 1.5.14 完整采用，669cb8a 运行路径已随 1.5.13 采用；不能再次回移或降级版本。

上次已发布审查到 8e1433d；本记录补入 11月23日已观察但未提交的四个提交，以及今天的四个提交，共八个。自目标 last_seen=669cb8a 之后累计23个新提交，只是已观察，均未凭本记录宣称集成。此前26个 pending 范围继续按 main 基线处理。

## 八个新增提交

| 提交 | 范围 | 同步前仍需验证 |
| --- | --- | --- |
| [534b047](https://github.com/rustdesk/rustdesk/commit/534b047525a2ac3cfea4cd2be5e551298b95f436) | Windows 导航键、Win/Menu 扩展键标志 | Windows 原生输入与远端键码、已有修饰键释放；没有把静态代码检查称为键鼠实测 |
| [be181a9](https://github.com/rustdesk/rustdesk/commit/be181a9d182491a5859fc1bf3ced99bc5aa86a8c) | TCP/WebSocket 保留错误原因，Mac EHOSTUNREACH 快速重试，hbb_common 子模块 | 严格 Key、rdoss_v1_ 过滤、开源服务端握手、加密初始化与错误链回归；不是所有 secure tcp 超时的通用修复 |
| [90f3fb1](https://github.com/rustdesk/rustdesk/commit/90f3fb136b6ea804643b6125ef9fe2d7bbc6551c) | Linux PipeWire 线性 DMA-BUF 回退、步长/裁剪及所有权 | Linux 原生 PipeWire、真实 portal/niri、fd 映射清理及打包加载；新增三个测试未在本轮运行 |
| [5a64e7a](https://github.com/rustdesk/rustdesk/commit/5a64e7a61ff88b81d576b1f7c5789343f26438d8) | Mac 摄像头及 nokhwa 原生依赖 | 实际授权拒绝/超时、摄像头权限门禁、旧 macOS 动态符号、两架构打包；能力/用途声明不等于 TCC 授权 |
| [66fabdf](https://github.com/rustdesk/rustdesk/commit/66fabdf305eff91385e4944c0565fa8ef5a7b730) | README 的 codec 所属目录 | 文档可单独回移；不为目录说明触发整套发版 |
| [9724512](https://github.com/rustdesk/rustdesk/commit/9724512429b053be04cdb843345e99ddcccdb80e) | 76文件、1193行增删；Linux 各包编入 DRM，运行时 allow-drm-capture 开关 | 不是简单打包修复，须独立候选验证服务端本机 IPC 认证、开关关闭/撤销、会话中切换、greeter/锁屏、配置同步、旧 glibc 回退及实际四种 Linux 包；不能照搬官方发布工作流 |
| [cc3300d](https://github.com/rustdesk/rustdesk/commit/cc3300d1bbe3e832e8882daf52df12fe5b2e0100) | 删除 SELinux 首页提示及54份语言中的旧键 | 保留维护者翻译政策，不顺带改已有翻译；该删除不等于 SELinux 系统策略失效 |
| [51f9439](https://github.com/rustdesk/rustdesk/commit/51f94394ce61e1f3d26229565c326b2862cbdd1f) | 删除未使用 SELinux Rust/手写 Web bridge 查询入口 | 配合前项检查 FFI 使用者，遵守本地禁止运行 codegen 的规则；不扩大到 Web 构建 |

11月23日四提交净范围15文件，含一个子模块更新；今天四提交净范围78文件，没有新的 Cargo.lock 或子模块更新。每提交完整文件清单和依赖精确提交范围见 review.json。文件数量是各自净范围，不是两段相加后的总范围。

## 依赖及权限边界

hbb_common 的 `fae47e0..450cc1d` 仅一个提交，改 tcp.rs/websocket.rs 保留底层 io/握手错误；没有 proto 或配置签名变化。子模块 pin 仍保持旧值。nokhwa 的 `c2f7466..f149b07` 七提交只涉及 Mac binding 与 AVFoundation backend，包含 NV12/420v 平面、5秒读取超时、配置锁释放和 iOS 专用符号条件编译。检查提交范围不能替代旧 macOS 实际 ABI 验收，未运行 Intel 包或访问摄像头。

Mac 摄像头改动还把既有 Quartz 屏幕 PixelBuffer 包装改为 Screen/Bytes 枚举，并新增 RGBA 字节路径；必须回归已有屏幕帧的数据、步长、格式与生命周期，不能把它称为只新增摄像头文件。权限请求有30秒超时，存在/创建仍要求实际 camera grant，服务端既有 ViewCamera 设备授权与 enable-camera 门禁必须保留。未声称摄像头默认开启，也不把摄像头授权当作屏幕录制授权。

Linux DRM 新方案默认编入 Flutter 包，运行时由开关控制；开关变化会重启本机 --server，已启动 producer 保留，关闭时拒绝新消费者。需确认已有流随重启确实终止，且关闭状态恢复现有 PipeWire/X11 路径。RPM 将 /usr/lib/rustdesk 的依赖自动提取排除，必须验证旧 glibc 无法加载可选库时的实际回退，不能只看安装依赖表。

9724512 在 src/ipc.rs 新增带 `#[tokio::main]` 的 sync_drm_capture_to_service_blocking。按本仓库规则，不直接引入在 helper 内隐藏创建 runtime 的入口；拥有者允许的例外只限手机共享进程显式启动，不扩大到 Linux 桌面。这是采用前需要设计清楚的兼容约束，不绕过检查来强行构建。core_main 的 Linux/DRM 提前初始化与自定义配置加载顺序也需重新跑新进程加密及签名校验。

上述改动尚未验证所有约束，保留现有正式版。PR #3 仍是原 head、无新增回复，旧 FD_FILESIZE 接收兼容风险未解决；不顺带合入、不重复构建它。

## 本次验证与回归面

只增加本目录两份审查资料。没有修改既有源文件、运行路径、主分支、版本号、工作流、基线、子模块或依赖 pin，没有触发原生测试/构建/Release。现有严格 Key、账号信令过滤、设备密码/人工授权/E2E、账号资料隔离、Mac 功能、Secrets 与确认式签名升级保留。没有改服务器、后台、密钥、防火墙、代理、本机应用或权限。移动 PR #17 的模拟器/编译结果不作为桌面七包验收。

已核对 main 文档与基线、现有构建为空、正式版及 PR 状态、八提交精确文件范围和两依赖提交范围，JSON 可解析及本次数据差异检查。没有将范围分诊称为完整代码审查或原生兼容通过。后续只有独立候选完整验证和同源码 main 正式构建七包、SHA256SUMS 与独立签名元数据通过后才发版、推进集成记录。
