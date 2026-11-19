# 私有移动端开发

移动端在独立 `codex/mobile-*` 分支验证，不改变桌面七包的发布集合、签名更新协议或当前正式版本。移动开发候选版本为 1.5.14+82；这不是已发布版本。

| 平台 | 本阶段实现与构建范围 | 尚未验收 |
| --- | --- | --- |
| Android ARM64、兼容 APK 的旧版鸿蒙 | 复用 Flutter/Rust 主控、MediaProjection 共享和辅助功能输入；独立应用 ID；候选只构建 unsigned APK | unsigned APK 已实际构建；启动、登录、远控和授权撤销尚未实测，最终持久签名安装包尚未生成 |
| iOS 15+ ARM64 模拟器 | 原有主控端；模拟器专用 Rust 静态库和依赖；真实安装/启动检查 | 实际构建/模拟器运行；手机屏幕共享；真机签名分发 |
| 原生 HarmonyOS 5/6 | 独立平台移植审查，先主控及软件解码，后按系统能力共享 | 尚无可安装 HAP，尚未完成核心引擎移植或真机验证 |

原生鸿蒙与 Android APK 是两个不同目标。公开 OpenHarmony SDK 可用于目标编译探索，不等同于已验证商业 HarmonyOS SDK、设备授权或应用分发。参考提案 rustdesk/rustdesk#16779 尚未合入；不得整份覆盖私有源码或替换共享子模块。iOS 当前没有被控引擎，不能把主控包称为已支持共享。iOS 普通应用和鸿蒙手机的全机输入控制不作为默认能力承诺。

## 构建边界

`.github/workflows/private-mobile-build.yml` 在移动候选分支 push 或手动触发时运行：测试配置校验、复用已有桥接生成任务，分别编译 Android ARM64 和 iOS ARM64 模拟器。仅上传开发产物，没有 Release 写权限，不发布 APK/HAP 到桌面 Release。长期 Android 签名密钥不进入候选构建；最终签名和移动发布须另设仅 main 的独立门禁，核验同一经测载荷及证书后再提供安装包。

应用 ID 为 `io.github.akurtlualgtm.rustdesk`。Android 保留 JNI namespace 以兼容原有 Rust JNI 符号；现有用户需安装独立应用并重新授权。iOS 模拟器包不能安装到真机，也不代表 App Store/TestFlight 可用。

服务器地址、服务器公钥与 HTTPS 账号 API 基址由 GitHub Secrets 注入；不写入公开源码或元数据。包内预设仍可被提取，Secrets 不隐藏二进制配置。新 HTTPS 预设沿用原有保存/覆盖选项优先规则，校验失败时使用不可用 HTTPS 地址，避免回退明文 API。保留中央 `rdoss_v1_` 信令过滤、严格 Key、设备密码/人工授权和端到端加密。后台账号及设备清单不是原生远控账号 ACL。

## 这次已有路径的回归面

- `src/common.rs` 和 `src/platform/mod.rs`：仅移动平台加载 HTTPS API 预设；桌面运行路径不变，纯校验测试在测试配置下可编译。
- `libs/scrap/build.rs`：仅 ARM64 iOS simulator 选择专用依赖 triplet，真机和桌面分支不变。
- iOS Xcode 项目：按 SDK 选择静态库；真机仍为原库路径。
- Android Gradle：私有构建可指定应用 ID；保持默认 namespace 和原应用 ID 回退；未提供 keystore 时允许 unsigned 开发候选。
- Android `InputService.kt` 和 Flutter `native_model.dart`：只删除移动端输入文本及设备资料日志，不改输入与会话行为。
- Cargo、Flutter、RPM 和桌面工作流的版本字段一致递增；桌面发布逻辑不变。

完整移动产物、源码来源、真实日志隐私和必要桌面回归全部验证前不合入 main；没有真实引擎和实际运行证据时不标为平台适配完成。iOS 共享、原生鸿蒙和最终安装包继续独立推进。


## 手机候选补充验证

手机候选适配上游 d40e1da715fa1694ff4712f394b81a6df6bcd612 的虚拟鼠标偏移修复：窗口坐标减去安全区和已有画布辅助栏偏移，与 InputModel 已有映射一致。只改变 FloatingMouse 移动/边缘滚动，关闭虚拟鼠标时仍是旧路径，不改直接触摸、键盘权限、仅查看、账号认证或加密。单个 VM widget 用例同时覆盖非零安全区和辅助栏；本地 ARM64 Flutter 的旧代码失败、修复后通过，临时本机 SDK 主题兼容改动已恢复，不提交。两个移动 CI 任务显式执行该 VM 用例；browser-only 的旧测试不作为此回归证据。固定 SDK 和真机仍须验收。

iOS 模拟器专用 libvpx 配置走 generic 软件 VP8/VP9，明确 simulator target/sysroot，避免旧 arm64-darwin 配置强制使用 iphoneos。旧真机、Android、桌面路径不变；保留 Rust dylib 链接和最终应用的 ARM64/IOSSIMULATOR/真实 Rust 符号/实际启动检查，不用只产静态库来掩盖错误。新增代码尚待下一轮 CI。

手机预览的最终发布入口需要 main 手动开启 publish_preview，且 Android/iOS模拟器/配置任务全部成功。候选分支不能得到长期 APK 签名密钥或发布 Release。最终任务核对同run的源码/身份/大小/SHA256，使用固定持久证书签名 Android，核对签名证书和签名前后所有非签名 ZIP 内容一致；私钥和密码在任何子进程启动前移出环境，只通过0600临时文件交给签名器。Android证书与服务器公钥、独立桌面更新Ed25519公钥是不同用途；本任务不获得桌面签名种子。

产物只进入独立 mobile-preview-vX 预发布：Android签名APK、iOS模拟器ZIP及摘要/来源记录，先创建精确源码tag再草稿上传/核验，最后公开为prerelease且make_latest=false，不能覆盖既有tag或桌面稳定latest。原生鸿蒙不在此预览集合；iOS模拟器仍不是真机安装包。上传失败或已有同名tag时干净失败并保留审查状态，不自动覆盖/删tag。该签名/发布入口尚未真实执行，不把纯测试通过当成已生成可安装包。

原生鸿蒙在独立codex/ohos-*草稿验证，未并入本候选。本分支尚无iOS手机共享/被控引擎，不宣称已经完成全部移动需求。所有必要移动与桌面候选验证通过前不进入main，不修改生产服务器、后台、密钥、防火墙、代理或本机权限。
