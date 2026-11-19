# iOS host core 编译移植（默认关闭、尚未实现共享）

新 ios-hostcore-compile-probe 只在独立分支把已有server/rendezvous/ipc模块纳入真实iOS SDK类型检查，没有新FFI、服务启动、监听、注册、用户权限或发布。首次检查预计会揭示上游把iOS排除后遗留的Android JNI、CM、camera/resolutions等阻点；失败不能当作平台适配或任何安全验证通过。

现有客户端未启用此feature，已有运行路径保持原条件；共享文件只改5个模块编译门禁和一个默认off feature。不能把该feature用于产品构建。只有补齐同一现有握手/密码/严格Key/E2E、只读协议和执行边界拒绝、首帧授权、可靠本地Close及必要loopback回归后，才可提供启动入口。不得为编译补成功空服务/弱化认证，不能调用Android JNI或模拟授权。

独立只读工作流仅自有codex/ios-hostcore-*，使用固定旧版iOS SDK/Xcode16.4/ARM64、同vcpkg/Rust版本、example.invalid合成配置，复用真实buffer测试与桥接前置，只有cargo check不运行host，不使用实际服务器或签名Secrets。旧版extension与新版iOS27 SCK前端另行验证；此工作流不证明extension可链接完整host或任何共享运行。
