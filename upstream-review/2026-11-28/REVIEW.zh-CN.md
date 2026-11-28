# 2026-11-28：API 空闲心跳及文件覆盖提示增量

上游从45e34a6新增两个提交，最新为104ef2a；正式版仍1.5.3。没有依赖或子模块变化，净范围仅三个文件。自main last_seen=669cb8a累计32个新提交，不能把它们或既有26个pending范围标为已集成。目标main与文档/基线不变，正式桌面版仍v1.5.14+82。

## 精确范围及待验证项

[5c680b4](https://github.com/rustdesk/rustdesk/commit/5c680b4d86c994a9de8a598c9afc7e5d3d5061bd)仅改libs/base/src/config/keys.rs与src/hbbs_http/sync.rs：新增api-idle-heartbeat-interval-seconds配置键，把空闲心跳从固定15秒变为15至300秒，空/无效输入仍15秒；记录发送URL，切换API时不受旧空闲间隔阻挡。已有连接时仍走原3秒循环，发送失败仍按发送尝试计时。上游新增两项测试（配置边界及切换URL）。现有heartbeat_url仍排除public API，字段id/uuid/ver/conns/modified_at及HTTPS账号Bearer与信令过滤入口不在补丁改动范围，不能据此声称真实账号后台已验收或实现原生ACL。

采用前须在实际模块运行新增测试及既有策略过滤、rdoss凭据/严格Key/加密回归，并核对配置切换、停止服务、已有连接和响应中的disconnect/strategy行为。自建账号会话撤销不能依赖未经确认的300秒轮询语义，独立后台兼容尚未实测；不改后台或服务端。此前c50ae97有关sysinfo预设地址簿密码的隐私候选仍待处理，本补丁没有替代它，也没有新增上传字段或恢复官方指纹的代码；不读取或发送真实设备配置来证明安全。

[104ef2a](https://github.com/rustdesk/rustdesk/commit/104ef2a86505ba4988dd7dca040aa92de49ab139)仅改libs/base/src/fs.rs：已有目标文件比较mtime时把本地和远端都按整秒比较，文件大小条件保留；新增一个500ms本地mtime的原生回归。get_current_digest已只发送整秒，NeedConfirm返回类型、resume的.digest/.download路径和Android文档入口未改。该is_identical标志会进入客户端覆盖确认/续传选择和Flutter提示，既有默认覆盖策略仍单独决定，不能把时间+大小称为内容哈希一致或声称所有覆盖操作已实测。

发布前应显式运行base包的实际测试，确认新增整秒用例、不同秒/大小的既有行为及上传/下载的覆盖提示/续传；private-build现有根crate、clipboard及scrap测试不能冒充base测试已执行。Windows/macOS/Linux文件时间行为与旧端协议兼容还需对应环境验证，不为这个小修复重构共享接口或更改文件协议。

## 本次检查与回归面

两个原始.patch保留各自上游逐文件原始hunk，仅补标准文件头。空闲心跳原始补丁干运行通过；文件时间原始补丁的测试hunk因目标缺少尚未采用的compressed_file_extensions_ignore_case上下文而失败。另提供-adapted.patch：生产改动及新增测试行逐字保持上游，只把新测试追加到目标既有测试模块末尾，不引入那个无关测试；该适配补丁干运行通过。SHA256、结果及精确父提交见review.json。git apply --check只证明可应用，未实际应用、编译或运行原生测试。候选回归面若将来采用，仅是配置键/空闲API心跳计时及普通文件路径的mtime等价提示；本轮这三条现有运行路径均没有改变。

继续同一个草稿PR #16，仅新增本目录五份审查数据；旧临时checkout已被清理，所以重新检出自己的审查分支，没有清理或重置用户工作区。PR #3 head和更新时间仍未变，不合入旧剪贴板风险、不重复构建原候选，也不顺带合入移动PR #17或此前Linux DRM/Mac camera范围。未递增版本、改main/工作流/基线/依赖pin或触发构建。权限、Key、设备授权/E2E、账号隔离、Mac定制、Secrets/关闭指纹及签名双确认升级保持；未改生产服务器、后台、密钥、防火墙、代理或本机应用与权限。

本次只核对精确源差异及调用者、目标文档/基线、构建为空、正式版/草稿状态、原始hunk一致、JSON/空白和已知服务器配置隐私；不把静态审查当作兼容性通过。只有必要原生和行为验证、候选七包及同源码main正式门禁全部通过后才能发布和推进已集成记录。
