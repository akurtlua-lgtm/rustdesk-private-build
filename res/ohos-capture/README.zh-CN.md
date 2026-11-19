# 原生鸿蒙只读采集准备实验

独立开发模块，不被现有 HAP、ArkTS/N-API 或 Rust 主控目标引用。CMake 开关 RD_BUILD_OHOS_CAPTURE_PROBE 默认关闭；没有创建、开始采集、启动 host 或输入控制入口。现有手机主控及其前后台行为保持不变。

frame.cpp 接收调用方提供的真实长度/行跨度和明确 BGRA/RGBA 格式，校验尺寸、截断/溢出和32MiB单帧上限，生成应用自有 packed BGRA；内存不足和错误分别返回错误值，不持有系统缓冲裸指针。其他格式拒绝，不能按请求RGBA推定实际内存布局，也不能把可能为0的AVBuffer容量当作映射内存有效长度。三个纯转换测试覆盖padding、RGBA通道及不合法/超大布局。

configure.cpp 仅对调用方持有的实例设置原始视频及系统回调。音频采样率/声道保持0，并显式检查关闭默认麦克风的API；逐项检查Init/State/Data/Error设置返回值，不调用Start。调用方负责实例和userData整个生命周期，任何失败都不能当作已准备好或已授权。只允许系统报告STARTED且收到有效首帧后再建立只读host，不能把Start返回0当作用户同意。

后续必须正确配对系统NativeBuffer引用及Map/Unmap，在回调内复制后才交给受授权的有界来源。停止必须先撤销host authority，再在非回调控制线程Stop/Release，不持有回调所需锁。公开SDK未保证Release后所有userData回调已结束；尚无真机验证，当前模块不管理或启动这些生命周期，禁止直接作为完整共享实现使用。

已在本机ARM64执行Clang AddressSanitizer/UndefinedBehaviorSanitizer三个转换测试，并使用固定公开OpenHarmony API20/6.0.0.47 Beta1实际交叉编译和链接ARM64共享库；这不等于商业HarmonyOS兼容、实际采集、系统授权、共享会话、后台或真机验收。没有华为开发者条件时先保留本开发范围。普通应用全机键鼠注入不在当前能力承诺内，后台账号/设备清单不替代远控密码或人工授权。
