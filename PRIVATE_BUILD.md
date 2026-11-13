# 自建 RustDesk 客户端

本仓库按拥有者授权公开，用于构建 Windows x64、macOS Intel/Apple Silicon、Linux x64 桌面客户端。
初始上游源码基线为 rustdesk/rustdesk 的 `95cca3f661f635b3cc85c04935fe8d96d8f432ef`，保留原有许可证；后续已集成增量见 `.github/upstream-baseline.json`。

## 安装包

[下载 Release 安装包](https://github.com/akurtlua-lgtm/rustdesk-private-build/releases)。首轮构建完成后才会出现安装包；[查看构建进度](https://github.com/akurtlua-lgtm/rustdesk-private-build/actions)。
Windows 选择 x86_64 的 MSI 或 EXE；macOS 根据处理器选择 x86_64 或 aarch64 DMG；Linux 根据发行版选择 DEB 或 RPM。
包未商业签名/Apple公证，安装需按系统提示处理。macOS 安装后需要授权屏幕录制和辅助功能。Linux Wayland 的权限与无人值守能力应在实际桌面环境另行验证。

## 构建配置

在本仓库 Settings → Secrets and variables → Actions 中设置：

| Secret 名称 | 内容 |
| --- | --- |
| RD_ID_SERVER | ID 服务器地址 |
| RD_RELAY_SERVER | 中继服务器地址及端口 |
| RD_SERVER_PUBLIC_KEY | 服务端公钥，不是私钥 |

这三个值通过编译环境变量注入原有 DEFAULT_SETTINGS 层，现有客户端配置优先。缺失配置会中止构建。本地编译也须设置同名环境变量。

公开源码及常规构建日志不显示配置值；预设地址和公钥仍包含在安装包中，不能把 Secrets 注入理解为隐藏安装包的配置。旧公开提交或第三方缓存不保证被撤回。
每台被控机须设置自己的强密码，公钥不是用户准入密码。本部署没有 Pro API。

## 相对上游的改动

- 中继回退 RequestRelay 信令携带 licence_key，兼容服务端严格 Key 校验；未修补的官方客户端可能中继失败。
- 更新仅检查本仓库正式稳定新版。支持新入口后，已安装的 Windows/macOS 客户端可在应用内下载、验证独立签名及 SHA256，确认后安装并重启；没有静默安装或倒计时确认。Linux/便携客户端保留手动安装。旧版首次仍需手动安装支持新入口的版本，关闭“启动时检查更新”也会停止定时检查。
- 预设上述私有服务器与公钥；没有宣称删除全部第三方请求，原有 STUN/IPv6 探测及可选功能仍需按需求审查。Linux 网关查找仍保留空 UDP、TTL=1 的既有探测；本次没有新增账号或屏幕数据上报。
- 构建沿用上游固定的工具链/构建步骤，仅保留手动桌面构建及其桥接/Windows辅助依赖。

服务端私钥绝不应进入此仓库。构建日志只使用公钥配置，不需要服务器SSH密钥或云平台凭据。

## 版本与正式发布

当前正式版为 `1.5.8+76`，已同步 macOS26 构建环境迁移并通过候选与正式两轮全平台构建。已安装 1.5.5/1.5.6/1.5.7 可通过新版提示自行升级；更旧版本可直接手动安装 1.5.8。M 系列 Mac 请选择 aarch64 DMG。详见 [v1.5.8](https://github.com/akurtlua-lgtm/rustdesk-private-build/releases/tag/v1.5.8)。

首个正式统一版本使用 `1.5.5+73`，`1.5.4` 保留为未完成候选。第一次须手动安装支持新版提示的 1.5.5 客户端；旧 `1.5.3` 无法收到此提示。

发布前同时更新 `Cargo.toml`、`libs/portable/Cargo.toml`、`Cargo.lock` 中的 rustdesk 和 rustdesk-portable-packer 条目、`flutter/pubspec.yaml`（递增构建号）、两个 `res/rpm-flutter*.spec`，以及 `.github/workflows/private-build.yml` 中的 `VERSION`、`TAG_NAME=v版本号`。补丁依次使用 `1.5.5`、`1.5.6` 等，不复用已发布版本。

预检会拒绝版本不一致、重复发布和降级，Windows 构建还会检查主程序、EXE 属性及 MSI 内部 ProductVersion。手动触发构建后，各平台先上传预发布包；只有 Windows、Mac 两种架构、Linux 全部成功且七个包的 SHA-256 均核对通过，流程才生成 `SHA256SUMS` 并将版本转为正式版。客户端只提示正式版，忽略草稿、预发布、相同或旧版本及其他仓库链接。

更新检查不带账号凭据或设备 ID。它需要访问 GitHub；无法访问时继续使用当前版本。

本版保留共用连接入口的独立账号凭据过滤修复（`rdoss_v1_`），覆盖全部桌面客户端。第一次从旧版升级后，应退出账号重新登录一次。Mac 桌面/全屏应用切换按钮也已保留。

## 应用内升级候选

1.5.10+78 目前是功能候选，当前正式版仍为 1.5.8。本候选支持已安装的 Windows EXE/MSI 和 Mac ARM/Intel 更新入口，下载完成后等待用户确认；服务端存在活动远控或无法确定会话状态时拒绝安装。Linux 保持手动安装。新增的 rustdesk-update.json 和 .sig 由独立更新密钥签名，不能替换为未签名的 SHA 清单。首次从 1.5.8 等旧客户端升级仍需手动安装一次。Mac 使用临时签名，升级后可能需要重新授权录屏、辅助功能等系统权限，尚未 Apple 公证。
