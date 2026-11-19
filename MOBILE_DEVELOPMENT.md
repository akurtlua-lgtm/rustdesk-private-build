# 私有移动端开发

移动基础适配已在独立分支及同源码桌面回归通过后进入 main，桌面正式版为 1.5.14+82；手机预览单独发布，不改变桌面七包集合及签名更新协议。手机共享与原生鸿蒙扩展仍在独立草稿，不能把桌面正式版等同于全部手机功能已完成。

| 平台 | 本阶段实现与构建范围 | 尚未验收 |
| --- | --- | --- |
| Android ARM64、兼容 APK 的旧版鸿蒙 | 复用 Flutter/Rust 主控、MediaProjection 共享和辅助功能输入；独立应用 ID；候选构建 unsigned APK，main 最终门禁持久签名；独立手机预览发布 | 实际发布 APK 已在 Android 17 ARM64 的 4KB/16KB 页面模拟器安装/启动；实际发布包的16KB模拟器系统共享启动/停止已验收；真机、辅助功能输入与端到端远控未验收 |
| iOS 15+ ARM64 模拟器 | 原有主控端；模拟器专用 Rust 静态库和依赖；真实安装/启动检查 | ARM64 应用已在 iOS 18.5 和 27.1 模拟器真实安装/启动；远控、手机屏幕共享、真机签名分发未验收 |
| 原生 HarmonyOS 5/6 | 独立平台移植审查，先主控及软件解码，后按系统能力共享 | 独立 PR #8 已用公开 OpenHarmony API20 SDK 编译真实 unsigned HAP 并检查引擎/ABI；未运行、签名或真机验收，不在当前 main/手机预览中 |

原生鸿蒙与 Android APK 是两个不同目标。公开 OpenHarmony SDK 可用于目标编译探索，不等同于已验证商业 HarmonyOS SDK、设备授权或应用分发。参考提案 rustdesk/rustdesk#16779 尚未合入；不得整份覆盖私有源码或替换共享子模块。iOS 当前没有被控引擎，不能把主控包称为已支持共享。iOS 普通应用和鸿蒙手机的全机输入控制不作为默认能力承诺。

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
