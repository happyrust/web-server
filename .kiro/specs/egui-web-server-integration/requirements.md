# Requirements Document

## Introduction

本需求文档定义了将 Web Server 集成到 EGUI 应用作为控制面板的功能需求，以及异地协同配置导入 API 的设计。该功能将使 EGUI 应用成为一个完整的运维控制中心，用户可以在同一界面中管理 Web Server、查看统计信息，并通过 URL 快速导入主站点配置。

当前系统已实现：
- 基于 egui 的异地协同运维界面（环境管理、拓扑配置、监控面板）
- 独立的 Web Server（Axum REST API + 文件服务）
- 基本的 Web Server 管理页面（启动/停止按钮）

需要新增的能力：
- Web Server 嵌入到 EGUI 应用进程中
- Web Server 统计信息展示（请求数、连接数、错误率）
- 配置导入 API（通过 URL 获取主站点配置）
- 一键导入配置功能

## Glossary

- **EmbeddedWebServer（嵌入式 Web 服务器）**: 运行在 EGUI 应用进程内的 Axum Web Server
- **ServerMetrics（服务器指标）**: Web Server 的运行统计数据，包括请求数、连接数、错误率等
- **ConfigImportAPI（配置导入 API）**: 提供站点配置信息的 REST API 端点
- **RemoteSyncConfig（异地协同配置）**: 包含环境、站点、MQTT、文件服务器等完整配置信息
- **ControlPanel（控制面板）**: EGUI 中的 Web Server 管理界面

## Requirements

### Requirement 1: Web Server 嵌入式集成

**User Story:** 作为运维人员，我希望 Web Server 运行在 EGUI 应用进程内，以便统一管理和监控

#### Acceptance Criteria

1. WHEN 运维人员启动 EGUI 应用，THE EguiApp SHALL 在后台线程启动嵌入式 Axum Web Server
2. WHEN Web Server 启动成功，THE EguiApp SHALL 在状态栏显示绿色指示器和监听地址
3. WHEN Web Server 启动失败，THE EguiApp SHALL 在界面显示错误信息，并提供重试按钮
4. WHEN 运维人员关闭 EGUI 应用，THE EguiApp SHALL 优雅关闭 Web Server，等待所有请求完成后退出
5. WHEN Web Server 运行时发生错误，THE EguiApp SHALL 捕获错误并在控制面板显示错误详情

### Requirement 2: Web Server 控制面板

**User Story:** 作为运维人员，我希望通过控制面板管理 Web Server 的启动和停止，以便灵活控制服务状态

#### Acceptance Criteria

1. WHEN 运维人员访问 Web Server 控制面板，THE EguiApp SHALL 显示服务器状态（运行中/已停止/错误）、监听地址、运行时长
2. WHEN 运维人员点击"启动服务器"按钮，THE EguiApp SHALL 在后台线程启动 Web Server，并在 5 秒内显示启动结果
3. WHEN 运维人员点击"停止服务器"按钮，THE EguiApp SHALL 发送停止信号到 Web Server，等待优雅关闭完成
4. WHEN 运维人员点击"重启服务器"按钮，THE EguiApp SHALL 先停止 Web Server，等待完全停止后再启动
5. WHEN Web Server 配置被修改，THE EguiApp SHALL 提示需要重启服务器才能生效

### Requirement 3: 服务器统计信息展示

**User Story:** 作为运维人员，我希望查看 Web Server 的实时统计信息，以便监控服务健康状态

#### Acceptance Criteria

1. WHEN 运维人员访问控制面板，THE EguiApp SHALL 显示总请求数、成功请求数、失败请求数、平均响应时间
2. WHEN 运维人员查看统计信息，THE EguiApp SHALL 显示当前活跃连接数、峰值连接数、总连接数
3. WHEN 运维人员查看错误统计，THE EguiApp SHALL 显示最近 10 条错误日志，包含时间、路径、错误类型、错误信息
4. WHEN 运维人员点击"刷新统计"按钮，THE EguiApp SHALL 从 Web Server 获取最新统计数据并更新界面
5. WHEN 统计数据更新，THE EguiApp SHALL 自动刷新图表（每 5 秒），显示请求数和响应时间的趋势曲线

### Requirement 4: 请求日志实时查看

**User Story:** 作为运维人员，我希望实时查看 Web Server 的请求日志，以便排查问题和审计操作

#### Acceptance Criteria

1. WHEN 运维人员访问日志查看区域，THE EguiApp SHALL 显示最近 100 条请求日志，包含时间、方法、路径、状态码、响应时间
2. WHEN 新的请求到达，THE EguiApp SHALL 自动追加日志到列表顶部，并高亮显示新日志
3. WHEN 运维人员点击日志行，THE EguiApp SHALL 在弹窗中显示完整的请求详情，包含请求头、请求体、响应头、响应体
4. WHEN 运维人员输入过滤条件（路径、状态码、时间范围），THE EguiApp SHALL 过滤日志列表，只显示匹配的日志
5. WHEN 运维人员点击"清空日志"按钮，THE EguiApp SHALL 清空内存中的日志缓存

### Requirement 5: 配置导入 API 设计

**User Story:** 作为运维人员，我希望主站点提供配置导入 API，以便其他站点快速获取配置信息

#### Acceptance Criteria

1. WHEN 主站点 Web Server 启动，THE WebServer SHALL 注册 GET /api/config/export 端点
2. WHEN 外部站点请求 /api/config/export，THE WebServer SHALL 返回 JSON 格式的配置信息，包含环境列表、站点列表、MQTT 配置、文件服务器配置
3. WHEN 配置信息包含敏感数据（密码、密钥），THE WebServer SHALL 过滤敏感字段，只返回公开信息
4. WHEN 请求包含 ?format=toml 参数，THE WebServer SHALL 返回 TOML 格式的配置文件
5. WHEN 请求包含 ?env_id=xxx 参数，THE WebServer SHALL 只返回指定环境的配置信息

### Requirement 6: 配置导入功能

**User Story:** 作为运维人员，我希望通过 URL 一键导入主站点配置，以便快速完成异地协同配置

#### Acceptance Criteria

1. WHEN 运维人员访问配置导入页面，THE EguiApp SHALL 显示 URL 输入框和"导入配置"按钮
2. WHEN 运维人员输入主站点 URL（如 http://main-site:3000/api/config/export）并点击"导入配置"，THE EguiApp SHALL 发送 HTTP GET 请求获取配置信息
3. WHEN 配置获取成功，THE EguiApp SHALL 解析 JSON 数据，并在预览面板显示将要导入的环境和站点列表
4. WHEN 运维人员勾选要导入的环境和站点并点击"确认导入"，THE EguiApp SHALL 调用本地 API 创建环境和站点
5. WHEN 导入过程中发生错误（网络错误、解析错误、创建失败），THE EguiApp SHALL 显示错误信息和失败的项目列表
6. WHEN 导入完成，THE EguiApp SHALL 显示导入摘要，包含成功数量、失败数量、跳过数量
7. WHEN 本地已存在同名环境或站点，THE EguiApp SHALL 提示用户选择覆盖或跳过

### Requirement 7: 配置导出功能

**User Story:** 作为运维人员，我希望导出当前站点配置为 URL，以便分享给其他站点

#### Acceptance Criteria

1. WHEN 运维人员访问配置导出页面，THE EguiApp SHALL 显示当前站点的配置导出 URL
2. WHEN 运维人员点击"复制 URL"按钮，THE EguiApp SHALL 复制 URL 到剪贴板，并显示成功提示
3. WHEN 运维人员点击"生成二维码"按钮，THE EguiApp SHALL 生成包含 URL 的二维码图片，方便移动设备扫描
4. WHEN 运维人员选择特定环境并点击"导出选中环境"，THE EguiApp SHALL 生成只包含该环境的配置 URL
5. WHEN 运维人员点击"测试 URL"按钮，THE EguiApp SHALL 访问自己的导出 API，验证配置是否可正常获取

### Requirement 8: 配置同步状态监控

**User Story:** 作为运维人员，我希望监控配置同步状态，以便确认配置已正确应用到所有站点

#### Acceptance Criteria

1. WHEN 运维人员访问同步状态页面，THE EguiApp SHALL 显示所有站点的配置版本号和最后同步时间
2. WHEN 运维人员点击"检查配置差异"按钮，THE EguiApp SHALL 对比本地配置和远程站点配置，显示差异项
3. WHEN 配置存在差异，THE EguiApp SHALL 高亮显示不一致的配置项，并提供"推送配置"按钮
4. WHEN 运维人员点击"推送配置"按钮，THE EguiApp SHALL 将本地配置推送到选中的远程站点
5. WHEN 推送完成，THE EguiApp SHALL 更新站点的配置版本号和同步时间

### Requirement 9: Web Server 性能监控

**User Story:** 作为运维人员，我希望监控 Web Server 的性能指标，以便及时发现性能瓶颈

#### Acceptance Criteria

1. WHEN 运维人员访问性能监控页面，THE EguiApp SHALL 显示 CPU 使用率、内存使用量、线程数、文件描述符数
2. WHEN 运维人员查看性能图表，THE EguiApp SHALL 显示最近 1 小时的性能趋势曲线（CPU、内存、请求数）
3. WHEN 性能指标超过阈值（CPU > 80%、内存 > 90%），THE EguiApp SHALL 在界面顶部显示警告横幅
4. WHEN 运维人员点击"导出性能报告"按钮，THE EguiApp SHALL 生成 CSV 格式的性能报告，包含时间序列数据
5. WHEN 运维人员设置性能告警阈值，THE EguiApp SHALL 保存阈值配置，并在超过阈值时触发告警

### Requirement 10: 配置验证和健康检查

**User Story:** 作为运维人员，我希望验证导入的配置是否正确，以便避免配置错误导致的服务故障

#### Acceptance Criteria

1. WHEN 运维人员导入配置后，THE EguiApp SHALL 自动执行配置验证，检查必填字段、格式正确性、端口冲突
2. WHEN 配置验证失败，THE EguiApp SHALL 显示验证错误列表，并阻止应用配置
3. WHEN 运维人员点击"健康检查"按钮，THE EguiApp SHALL 测试 MQTT 连接、文件服务器连接、数据库连接
4. WHEN 健康检查完成，THE EguiApp SHALL 显示检查结果，包含成功项、失败项、警告项
5. WHEN 健康检查发现问题，THE EguiApp SHALL 提供修复建议和快速修复按钮
