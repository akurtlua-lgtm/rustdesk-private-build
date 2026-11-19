# iOS 屏幕共享：扩展链接门禁

本分支先验证真实 Broadcast Upload Extension 能否链接现有 Rust 核心，不启动 Flutter engine；不是可用的手机共享功能，不包含主应用入口、采集传输、App Group/共享 Keychain 或 host 认证服务。扩展收到启动请求会立即明确失败，不录屏、不开放连接或输入。

默认关闭的 `ios-host-link-probe` feature 只增加独立加密 roundtrip C 入口。已有 `src/lib.rs` 只新增该模块门禁，默认发行路径没有改动；Cargo 不增加依赖，单个单元用例检查初始化后可完成认证加密。

隔离工作流只在自己的 `codex/ios-host-*` 分支运行，固定 Xcode16.4、Rust1.81、vcpkg 基线和 ARM64；分别构建模拟器/设备 SDK 的真实 extension，开启 application-extension-only 检查。确认平台、ABI、Rust 入口、无 Flutter/Dart 或应用专属 API 依赖后仅保存 unsigned 开发 Artifact，不安装、不签名、不发布。全程只用 example.invalid 合成配置，没有服务器或长期更新/安装签名 Secrets。

下一阶段才能接有界采集缓冲、默认关闭的只读 host、现有 Key/密码/E2E 和强制本地关闭连接；主应用只承载系统共享 picker 与配置，host 在广播扩展进程运行。账号 token 不进入共享容器。人工审批没有可靠跨进程界面时应拒绝该模式；停止录屏后必须取消本地 transport，不能仅向对端发送 stop 消息。

当前门禁尚未执行通过，不能把编译计划、合成帧或静态检查称作录屏授权、跨应用/后台共享或真机验收。这个独立分支不合入当前手机候选，不修改生产服务器、账号后台、权限、密钥或桌面更新流程。
