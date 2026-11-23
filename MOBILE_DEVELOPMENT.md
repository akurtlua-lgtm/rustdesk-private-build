# 私有移动端基础候选记录

以下保留早期平台基础记录；新共享链路的当前候选边界见 IOS_SHARING_PROGRESS.md、IOS_CAPTURE_PROGRESS.md（独立采集分支）与 OHOS_SHARING_PROGRESS.md。Android 正式版已另行发布 android-v1.5.14；本文中的早期 unsigned/待发布描述不代表当前正式版状态。

移动基础适配已在独立分支及同源码桌面回归通过后进入 main，桌面正式版为 1.5.14+82；手机预览单独发布，不改变桌面七包集合及签名更新协议。手机共享与原生鸿蒙扩展仍在独立草稿，不能把桌面正式版等同于全部手机功能已完成。

| 平台 | 本阶段实现与构建范围 | 尚未验收 |
| --- | --- | --- |
| Android ARM64、兼容 APK 的旧版鸿蒙 | 独立正式 Android 包 android-v1.5.14，沿用 Flutter/Rust 主控、MediaProjection 与辅助功能输入 | 拥有者已确认 Android 设备连接及操作正常；不据此声称旧版鸿蒙所有机型均已验收 |
| iOS 15+ ARM64 模拟器 | 原有主控已在 iOS 18.5、27.1 模拟器安装/启动；本候选另接入默认关闭的系统共享界面与广播扩展 | 共享集成仍须当前源码完整 SDK/应用回归；系统录屏、真机签名分发及端到端手机共享未验收 |
| 原生 HarmonyOS 5/6 | 原生 ArkTS 主控、Rust 引擎、渲染、音频及独立账号资料；本候选另接入默认关闭的系统共享面板 | 公开 API20 的 unsigned HAP 已编译验证；当前集成须同源码复验，商业系统、签名、采集授权和真机远控未验收 |

原生鸿蒙与 Android APK 是两个不同目标。公开 OpenHarmony SDK 可用于目标编译探索，不等同于已验证商业 HarmonyOS SDK、设备授权或应用分发。参考提案 rustdesk/rustdesk#16779 尚未合入；不得整份覆盖私有源码或替换共享子模块。已发布 iOS 主控包不包含本候选共享入口，不能把该包称为已支持共享。iOS 普通应用和鸿蒙手机的全机输入控制不作为默认能力承诺。

## 构建边界

`.github/workflows/private-mobile-build.yml` 在移动候选分支 push 或手动触发时运行：测试配置校验、复用已有桥接生成任务，分别编译 Android ARM64 和 iOS ARM64 模拟器。候选只上传开发产物，没有 Release 写权限，不发布 APK/HAP 到桌面 Release。长期 Android 签名密钥不进入候选构建；已设置仅 main 手动启用的最终签名和独立手机预览门禁，核验同一 run 的经测载荷及证书后才提供安装包。

应用 ID 为 `io.github.akurtlualgtm.rustdesk`。Android 保留 JNI namespace 以兼容原有 Rust JNI 符号；现有用户需安装独立应用并重新授权。iOS 模拟器包不能安装到真机，也不代表 App Store/TestFlight 可用。

服务器地址、服务器公钥与 HTTPS 账号 API 基址由 GitHub Secrets 注入；不写入公开源码或元数据。包内预设仍可被提取，Secrets 不隐藏二进制配置。新 HTTPS 预设沿用原有保存/覆盖选项优先规则，校验失败时使用不可用 HTTPS 地址，避免回退明文 API。保留中央 `rdoss_v1_` 信令过滤、严格 Key、设备密码/人工授权和端到端加密。后台账号及设备清单不是原生远控账号 ACL。

## 这次已有路径的回归面

- `src/common.rs` 和 `src/platform/mod.rs`：仅移动平台加载 HTTPS API 预设；桌面运行路径不变，纯校验测试在测试配置下可编译。
- `libs/scrap/build.rs`：仅 ARM64 iOS simulator 选择专用依赖 triplet，真机和桌面分支不变。
- iOS Xcode 项目：按 SDK 选择静态库；真机仍为原库路径。
- Android Gradle：私有构建可指定应用 ID；保持默认 namespace 和原应用 ID 回退；未提供 keystore 时允许 unsigned 开发候选。
- Android `InputService.kt` 和 Flutter `native_model.dart`：只删除移动端输入文本及设备资料日志，不改输入与会话行为。
- Cargo、Flutter、RPM 和桌面工作流的版本字段一致递增；桌面发布逻辑不变。

移动基础适配已完成上述 main 合入门禁；后续扩展仍须完整验证后才能合入，没有真实引擎和实际运行证据时不标为平台适配完成。iOS 共享、原生鸿蒙和最终安装包继续独立推进。


## 原生鸿蒙开发候选

`harmonyos/` 是独立的 Stage/ArkTS V2 原生工程，ARM64、最低 API17、目标 API20，应用 ID 与私有移动端一致。公开 SDK 的真实元数据为 6.0.0.47 / Beta1；镜像目录名为 Release 不代表组件已获商业 HarmonyOS 真机验收。当前只使用普通应用能力，不声明手机全机输入注入权限、不提供伪成功接口。参考提案只用于评估，没有合入其790文件或替换共享协议历史。

真实 Rust 目标检查和 debug 动态库链接已通过；完整 HAP 已执行 CompileArkTS、BuildNative、PackageHap、PackingCheck，并独立核对应用身份/API、真实 ABC、三个 ARM64 ELF、14个原生入口和依赖。原生核心只依赖 OHAudio、libc++ 和 libc；未链接 X11/GTK/PulseAudio。桥接编译使用实际 SDK 的 `-Wall -Wextra -Werror`，Clang 静态分析无诊断。控制事件复制后通过有界非阻塞 TSFN 送 UI；RGBA 同步绘制后解绑 EGL 上下文。退出先注销并等待不可重入回调结束，再释放 UI 资源；输入入口同时要求真实首帧、认证连接、远端键鼠许可及非仅查看，初始会话明确为仅查看。

账号请求只在线程池工作线程执行，固定 HTTPS 接口、证书校验、拒绝重定向、超时与响应上限；token 仅保留内存，401清除账号/设备资料，账号变化清除设备编辑表单。注册等待管理员审核；账号登录与设备密码/2FA/人工授权分离。普通用户设备范围仍由原后台校验；本阶段没有新增生产后台路由，设备资料审核/归档不表示原生远控 ACL。尚未发送真实账号请求做整端运行验证。

`.github/workflows/private-ohos-build.yml` 单独在 `codex/ohos-*` push 或手动触发，验证固定 SDK 摘要及实际组件版本，编译 release Rust/HAP并检查产物，只上传 unsigned 开发 Artifact，无 Release 写权限。长期签名或服务器私钥不进入本流程。构建须在隔离 checkout 执行；脚本会应用已核对的共享库补丁并产生本地依赖/构建缓存，遇到其他 tracked 修改会拒绝，不重置、清理或提交任何现有工作区。

共享库 gitlink 固定原 fae47e024dd83cef6aaecde0aa7d11731df11649，仅在隔离构建应用 `res/ohos/hbb_common-ohos.patch`：分离 X11 依赖/导出，并让配置使用应用私有目录。两个本地兼容副本分别固定原已锁定 rdev 0904f23、webrtc-util 49c89bd；保留来源/许可证，除 OHOS 的依赖与 cfg 隔离外不改变原平台源代码。不能因此宣称既有桌面和 Android/iOS 全部回归通过。

已有文件的必要回归面：

- `Cargo.toml`/`Cargo.lock`、`libs/enigo/Cargo.toml`：固定兼容副本，非 OHOS 源码保持原提交，仍须全平台依赖和编译回归。
- `libs/base/Cargo.toml`、`libs/base/src/platform/mod.rs`：OHOS 不使用桌面 Linux 组件；其他平台原条件保持。
- `libs/scrap/Cargo.toml`、`libs/scrap/build.rs`、`libs/scrap/src/common/{codec,convert,mod}.rs`、`res/vcpkg/libvpx/portfile.cmake`：新增 OHOS 交叉编译、软件解码/RGBA和专用依赖目标；未将平台称为桌面 Linux。
- `src/lib.rs`、`src/platform/mod.rs`、`src/platform/private_mobile.rs`、`src/common.rs`：选择原生控制端入口和应用目录/HTTPS预设，排除 OHOS 不支持的桌面服务；其他平台使用原路径。
- `src/client.rs`、`src/client/{io_loop,file_trait,screenshot}.rs`：OHOS 独立音频/RGBA接线，排除桌面剪贴板/窗口/屏幕采集路径；共享循环仅增加 OHOS 活动会话检查，防止关闭后延迟启动。
- `src/keyboard.rs`、`src/keyboard/shortcuts.rs`、`src/ui_{interface,cm_interface,session_interface}.rs`：仅 OHOS 隔离本地全局键盘/宿主服务接口，保留既有远端会话认证与输入方法；没有变更共享 trait/调用者签名。
- `src/auth_2fa.rs`、`src/hbbs_http/sync.rs`、`src/lan.rs`、`src/lang.rs`、`src/privacy_mode.rs`：OHOS 使用已有移动端配置分支或排除宿主功能；原平台条件与实现保持。
- `.github/scripts/verify_mobile_artifacts.py`、`test_mobile_artifacts.py`：新增 HAP 校验和拒绝错误身份/内嵌构建路径的回归；原 APK/iOS 校验函数保持。

新增实现集中于 `src/ohos_ffi.rs`、`src/client/ohos_audio.rs`、`src/platform/ohos_account.rs`、`harmonyos/` 与独立构建脚本。没有修改桌面正式发布逻辑、服务器、账号后台、密钥、防火墙、代理或本机权限。

已知未通过/未完成：Rust Clippy 的 `-D warnings` 仍被共享库42项既有 lint 阻断，不能称为 Clippy 全绿；该角色审查因此停止，另行的代码/安全/C++/UI静态审查不替代此检查。尚无原生 OHOS 运行环境/手机，未验证实际初始化、TLS请求、画面、音频、输入权限拒绝与退出压力；release HAP/全平台回归仍须最终候选 CI。iOS 手机屏幕共享/被控核心和原生鸿蒙手机采集均尚未实现，不把主控编译成功当作完成全部手机需求。

## 手机候选补充验证

手机候选适配上游 d40e1da715fa1694ff4712f394b81a6df6bcd612 的虚拟鼠标偏移修复：窗口坐标减去安全区和已有画布辅助栏偏移，与 InputModel 已有映射一致。只改变 FloatingMouse 移动/边缘滚动，关闭虚拟鼠标时仍是旧路径，不改直接触摸、键盘权限、仅查看、账号认证或加密。单个 VM widget 用例同时覆盖非零安全区和辅助栏；本地 ARM64 Flutter 的旧代码失败、修复后通过，临时本机 SDK 主题兼容改动已恢复，不提交。两个移动 CI 任务显式执行该 VM 用例；browser-only 的旧测试不作为此回归证据。固定 SDK 和真机仍须验收。

iOS 模拟器专用 libvpx 配置走 generic 软件 VP8/VP9，明确 simulator target/sysroot，避免旧 arm64-darwin 配置强制使用 iphoneos。旧真机、Android、桌面路径不变；保留 Rust dylib 链接和最终应用的 ARM64/IOSSIMULATOR/真实 Rust 符号/实际启动检查，不用只产静态库来掩盖错误。该代码已通过移动候选实际构建及 iOS 模拟器启动检查。

手机预览的最终发布入口需要 main 手动开启 publish_preview，且 Android/iOS模拟器/配置任务全部成功。候选分支不能得到长期 APK 签名密钥或发布 Release。最终任务核对同run的源码/身份/大小/SHA256，使用固定持久证书签名 Android，核对签名证书和签名前后所有非签名 ZIP 内容一致；私钥和密码在任何子进程启动前移出环境，只通过0600临时文件交给签名器。Android证书与服务器公钥、独立桌面更新Ed25519公钥是不同用途；本任务不获得桌面签名种子。

产物只进入独立 mobile-preview-vX 预发布：Android签名APK、iOS模拟器ZIP及摘要/来源记录，先创建精确源码tag再草稿上传/核验，最后公开为prerelease且make_latest=false，不能覆盖既有tag或桌面稳定latest。原生鸿蒙不在此预览集合；iOS模拟器仍不是真机安装包。上传失败或已有同名tag时干净失败并保留审查状态，不自动覆盖/删tag。该门禁已真实执行并发布 mobile-preview-v1.5.14；具体来源、签名和验收范围见下方发布记录。

原生鸿蒙在独立codex/ohos-*草稿验证，未并入本候选。本分支尚无iOS手机共享/被控引擎，不宣称已经完成全部移动需求。所有必要移动与桌面候选验证通过前不进入main，不修改生产服务器、后台、密钥、防火墙、代理或本机权限。


私有手机构建显式启用 RD_PRIVATE_MOBILE_MANUAL_UPDATES：启动时不再请求桌面 stable latest，也隐藏旧手机更新横幅和启动检查开关，防止桌面版本较高时误报并跳到官方客户端下载。手机预览只提供手动下载，不默认订阅预发布或自动安装。开关默认关闭，桌面、Web与原非私有客户端继续原更新路径。新增单个 VM 用例验证手机入口在访问原生更新后端/注册Timer前返回，两移动CI显式开启参数执行；本地旧行为失败、修复后与鼠标用例共2项通过，临时本机SDK兼容编辑已恢复。


## 已核验的构建与边界（2026-11-19）

移动候选源码 `5d8a17ba9c85f279b857ccface5843292c882de3`、run 44475602661 的 Android/iOS 两份完整载荷、API 摘要、应用身份、ARM64 平台、真实 Rust 符号及实际日志隐私通过独立核验；每个移动任务均执行两项 VM 回归。桌面同源码候选及正式 run 44499127062 的七包与独立更新签名均通过。iOS 27.1 兼容验证是同一固定客户端载荷的另外一次模拟器测试，不冒充新的客户端编译来源。

固定候选 5d8/run44475602661 以临时测试证书在 4KB/16KB 页面 Android 17 ARM64 模拟器完成系统 MediaProjection 正常授权启动/停止，辅助功能未开启、远程文件访问跳过，没有远程对端。实际 main 发布包的持久签名、完整载荷与来源已独立核验，并在两种页面环境实际安装、启动且存活15秒；同一发布APK又在16KB模拟器经过系统确认启动/停止采集，输入辅助功能保持关闭、远程文件访问跳过。这不能替代真机键鼠、远控会话或旧版鸿蒙 APK 兼容性验收。四个实际 ELF 的 16KB 对齐静态检查也通过。

iOS 共享原型的真实 SDK 编译、合成图像 VP8/加密往返及来源关闭测试均在独立 PR #10–12；不包含实际系统采集或已开放的手机被控入口。iOS 27 的 ScreenCaptureKit 与旧系统 ReplayKit 路径分别验证，模拟器不支持的系统采集能力明确关闭。公开 SDK 的鸿蒙 HAP 编译通过不代表商业系统授权、真机运行或手机共享已完成。

手机预览签名脚本修复单行密码文件被重复消费的问题，使用相同 Build Tools 36.0.0 和临时证书复现旧命令 EOF、验证修复后 v1/v2/v3 签名及全部非签名 ZIP 内容不变；未在本机读取长期发布密钥。main 实际发布及独立完整核验均已通过，长期签名密钥只在 main 最终任务使用，没有在本机读取。


## 手机预览 1.5.14+82 已发布

[下载独立手机预览](https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/tag/mobile-preview-v1.5.14)。该版本是 prerelease，手动下载安装，不替换桌面稳定 latest。Android ARM64 为持久证书签名 APK；iOS 为 ARM64 模拟器 ZIP，不能安装到 iPhone，当前没有 Apple 开发者签名分发。原生鸿蒙不在此集合。

精确源码为 `8373fa77f8a662edb15b73c5cf4eca9faee04c08`，run44514425331 六任务全部成功。两份完整构建归档 API 摘要/源码/run/内部载荷、Android应用身份及四个ARM64 ELF、iOS真实Rust符号与21个Mach-O、实际日志隐私、10项Python和每平台2项VM回归均独立核验。实际发布四附件的API大小/SHA256、SHA256SUMS/MOBILE_BUILD、精确来源tag、Android固定证书及v1/v2/v3签名、签名前后所有非签名ZIP条目内容、iOS载荷完全不变均通过。实际发布APK再次在4KB和16KB ARM64 Android模拟器启动并存活15秒，截图和界面检查通过；16KB环境下的系统画面采集正常授权启动/停止也通过，不宣称已完成真机或远控会话。

该手机构建来源包含签名脚本/发布后文档提交，桌面 v1.5.14 的源码标签仍为5d8，未移动、重建或混淆。桌面稳定latest及七包Ed25519更新协议保持不变。

原生鸿蒙PR #8已用固定公开SDK生成真实unsigned HAP并核验ABI；PR #14的默认关闭独立采集准备库已通过3项ASan/UBSan行为测试和真实API20 ARM64 SDK编译/链接，未进入HAP或提供Start入口。iOS PR #11的BGRA/NV12及方向转换通过真实SDK与模拟器2项CoreVideo合成测试，实际系统采集仍未接入；PR #12的真实iOS18.5主库编译/链接、2项合成媒体与2项加密TCP设备密码/TOTP/只读/视频/Scope撤销回归已通过，但注册及系统采集生命周期尚未接通，实验功能不能用于产品。


本次发布的同一iOS载荷（8373/run44514425331，SHA256为1b1e858f8c6a28f7e6787a90cb6a206b6d05e0034e99a24874629bf98889443e）在iOS27.1模拟器实际安装/存活15秒及2项CoreVideo转换复验通过，测试source1fe7a0e0db0c51cd6a82acb12307b6dd8067a1c8/run44524698491。首轮在安装应用前模拟器启动超时；仅同源码重跑失败任务一次，未更改安装包、检查或超时，第二次成功。四份历史任务日志及原失败记录保留核验，不能把首轮当通过。
