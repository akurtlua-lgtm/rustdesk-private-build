# 自建 RustDesk 客户端

本仓库按拥有者授权公开，用于构建 Windows x64、macOS Intel/Apple Silicon、Linux x64 桌面客户端。
上游源码固定于 rustdesk/rustdesk 的 `95cca3f661f635b3cc85c04935fe8d96d8f432ef`，保留原有许可证。

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
- 统一更新入口不再请求官方版本接口，清空可用更新地址；后续从本仓库人工更新。
- 预设上述私有服务器与公钥；没有宣称删除全部第三方请求，原有 STUN/IPv6 探测及可选功能仍需按需求审查。
- 构建沿用上游固定的工具链/构建步骤，仅保留手动桌面构建及其桥接/Windows辅助依赖。

服务端私钥绝不应进入此仓库。构建日志只使用公钥配置，不需要服务器SSH密钥或云平台凭据。
