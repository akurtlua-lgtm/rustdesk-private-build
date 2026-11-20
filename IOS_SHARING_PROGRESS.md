# iOS 手机共享接入候选

本候选仍默认关闭，不进入已发布的 Android、iOS 主控或桌面安装包。新增代码使用现有开源 hbbs 的 UDP RegisterPk/RegisterPeer 协议及 TCP 中继；设备公钥确认之后，还须由当前 UDP socket 的 RegisterPeerResponse 确认才能显示就绪。没有引入商业服务端或修改服务端。

共享停止撤销 Scope，并关闭采集来源。注册工作和连接建立前的等待响应撤销；已经进入现有 Connection 的连接保留其本地强制关闭、会话缓存和订阅清理路径。旧工作退出之前不能准备新的共享 Scope。调用者应释放采集授权对象并等待注册工作完成，不得用 abort 代替正常停止。

中继信令沿用原生 RequestRelay licence_key；设备连接仍强制现有签名身份握手和端到端加密。现有设备密码、TOTP 及只读协议门禁不变；主控键鼠、文件、音频和其他未支持操作仍被拒绝。独立 HTTPS 账号及设备清单不是原生远控 ACL。

新增根库合成网络回归以本机 UDP 和 TCP 端点模拟现有 OSS 协议，检查当前地址确认前不就绪、hbbr Key、实际设备 SignedId 握手、停止后 EOF、注册工作回收及下一次 Scope。既有两项真实加密密码/TOTP/只读测试保留。此文件不代表 SDK 构建或系统采集已经通过，结果须以当前源码 CI 为准。

回归面：`src/server/mobile_host.rs` 只改变新 opt-in Scope 的寿命和注册状态；`src/rendezvous_mediator.rs` 仅新增 iOS+feature 模块门禁；新 `src/rendezvous_mediator/mobile_sharing.rs` 包含手机专用 UDP/TCP 共享路径。桌面、Android、feature 关闭时均不进入该新路径。测试文件只追加一个行为用例，执行器要求两项媒体和三项根库测试真实通过。

此新共享路径暂只提供 UDP 信令与 TCP 中继，不启用 WebRTC、打洞、LAN 或仅 TCP/WebSocket/SOCKS 代理；不影响已有主控端的这些连接方式。系统采集前端、应用现有异步执行入口及原生鸿蒙被控引擎尚待连接。没有创建或嵌套 Tokio runtime，也不把合成网络测试当作系统录屏或真机测试。
