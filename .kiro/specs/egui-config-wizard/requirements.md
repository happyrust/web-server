# Requirements Document

## Introduction

本需求文档定义了为 egui 原生界面添加配置向导功能的需求。该功能将参考 frontend/v0-aios-database-management 的任务创建向导，为 egui 应用提供类似的配置解析和模型生成任务的能力，并支持数据库启动管理。

当前系统已实现：
- 基于 egui 的原生 GUI 界面（src/gui/）
- 环境和站点配置管理
- Web Server 启动管理（src/gui/pages/web_server.rs）
- 站点配置界面（src/gui/pages/site_config.rs）

需要新增的能力：
- 任务创建向导界面（类似 frontend 的 TaskCreationWizard）
- 数据解析任务配置（DataParsingWizard）
- 模型生成任务配置（ModelGeneration）
- 空间树生成任务配置（SpatialTreeGeneration）
- 数据库启动和管理界面
- 任务监控和进度显示

## Glossary

- **TaskCreationWizard（任务创建向导）**: 分步骤引导用户创建和配置各类数据处理任务的界面组件
- **DataParsingWizard（数据解析任务）**: 解析 PDMS 数据库文件，提取几何和属性信息的任务类型
- **ModelGeneration（模型生成任务）**: 基于解析数据生成 3D 模型和网格文件的任务类型
- **SpatialTreeGeneration（空间树生成任务）**: 构建空间索引树以优化查询性能的任务类型
- **SurrealDB**: 项目使用的图数据库，需要启动和管理
- **DbOption.toml**: 数据库和应用配置文件
- **TaskRequest**: 任务请求数据结构，包含任务类型、参数和目标站点信息

## Requirements

### Requirement 1: 任务创建向导主界面

**User Story:** 作为运维人员，我希望通过向导式界面创建数据处理任务，以便快速配置和启动解析、生成等操作

#### Acceptance Criteria

1. WHEN 运维人员访问任务创建页面，THE EguiConfigWizard SHALL 显示任务创建向导，包含步骤指示器（基础信息/选择站点/任务参数/预览确认）
2. WHEN 运维人员在步骤 1 填写任务名称和选择任务类型，THE EguiConfigWizard SHALL 验证任务名称非空且唯一，并根据任务类型显示对应的图标和描述
3. WHEN 运维人员选择任务优先级，THE EguiConfigWizard SHALL 提供四个选项（低/普通/高/紧急），默认选中"普通"
4. WHEN 运维人员点击"下一步"按钮，THE EguiConfigWizard SHALL 验证当前步骤的必填字段，验证通过后切换到下一步骤
5. WHEN 运维人员点击"上一步"按钮，THE EguiConfigWizard SHALL 保留已填写的数据并返回上一步骤

### Requirement 2: 数据解析任务配置

**User Story:** 作为运维人员，我希望配置数据解析任务参数，以便指定要解析的数据库范围和解析选项

#### Acceptance Criteria

1. WHEN 运维人员选择"数据解析任务"类型，THE EguiConfigWizard SHALL 在参数配置步骤显示解析模式选择器（全部解析/指定数据库编号/指定参考号）
2. WHEN 运维人员选择"指定数据库编号"模式，THE EguiConfigWizard SHALL 显示数据库编号输入框，支持逗号分隔的多个编号（如 7999,8001,8002）
3. WHEN 运维人员选择"指定参考号"模式，THE EguiConfigWizard SHALL 显示参考号输入框，支持单个参考号输入
4. WHEN 运维人员填写解析参数，THE EguiConfigWizard SHALL 实时验证输入格式（数据库编号为数字，参考号符合格式要求）
5. WHEN 运维人员完成参数配置，THE EguiConfigWizard SHALL 在预览步骤显示解析范围摘要（如"解析数据库: 7999, 8001, 8002"）

### Requirement 3: 模型生成任务配置

**User Story:** 作为运维人员，我希望配置模型生成任务参数，以便控制生成选项和性能参数

#### Acceptance Criteria

1. WHEN 运维人员选择"模型生成任务"类型，THE EguiConfigWizard SHALL 在参数配置步骤显示生成选项复选框（3D 模型/网格/空间树/布尔运算）
2. WHEN 运维人员配置网格容差比例，THE EguiConfigWizard SHALL 提供滑块控件，范围 0.001-1.0，默认值 0.01，并显示当前数值
3. WHEN 运维人员配置最大并发数，THE EguiConfigWizard SHALL 提供数字输入框，范围 1-32，默认值 4
4. WHEN 运维人员启用并行处理开关，THE EguiConfigWizard SHALL 显示并行处理相关的高级选项（线程池大小、批处理大小）
5. WHEN 运维人员完成参数配置，THE EguiConfigWizard SHALL 在预览步骤显示生成选项摘要和性能参数

### Requirement 4: 空间树生成任务配置

**User Story:** 作为运维人员，我希望配置空间树生成任务，以便优化空间查询性能

#### Acceptance Criteria

1. WHEN 运维人员选择"空间树生成任务"类型，THE EguiConfigWizard SHALL 在参数配置步骤显示空间树配置选项
2. WHEN 运维人员配置树深度参数，THE EguiConfigWizard SHALL 提供滑块控件，范围 1-10，默认值 5
3. WHEN 运维人员配置节点容量参数，THE EguiConfigWizard SHALL 提供数字输入框，范围 10-1000，默认值 100
4. WHEN 运维人员选择空间索引类型，THE EguiConfigWizard SHALL 提供下拉选择器（R-Tree/Quad-Tree/Oct-Tree），默认选中 R-Tree
5. WHEN 运维人员完成参数配置，THE EguiConfigWizard SHALL 在预览步骤显示空间树配置摘要

### Requirement 5: 站点选择和验证

**User Story:** 作为运维人员，我希望选择任务的目标站点，以便将任务分配到正确的部署环境

#### Acceptance Criteria

1. WHEN 运维人员进入站点选择步骤，THE EguiConfigWizard SHALL 显示可用站点列表，包含站点名称、环境、地区和状态指示器
2. WHEN 运维人员选择站点，THE EguiConfigWizard SHALL 高亮显示选中的站点，并在右侧显示站点详细信息（HTTP 地址、数据库编号、SurrealDB 配置）
3. WHEN 运维人员点击"测试连接"按钮，THE EguiConfigWizard SHALL 调用站点连接测试 API，并在 3 秒内显示测试结果（成功/失败、延迟、错误信息）
4. WHEN 站点连接测试失败，THE EguiConfigWizard SHALL 显示警告提示，但仍允许用户继续创建任务
5. WHEN 运维人员未选择站点，THE EguiConfigWizard SHALL 禁用"下一步"按钮并显示提示信息

### Requirement 6: 任务预览和确认

**User Story:** 作为运维人员，我希望在创建任务前预览完整配置，以便确认所有参数正确无误

#### Acceptance Criteria

1. WHEN 运维人员进入预览步骤，THE EguiConfigWizard SHALL 显示任务配置摘要，包含任务名称、类型、优先级、目标站点和所有参数
2. WHEN 运维人员查看资源需求预估，THE EguiConfigWizard SHALL 显示预计内存使用、磁盘空间需求和预计耗时
3. WHEN 运维人员查看注意事项，THE EguiConfigWizard SHALL 显示任务类型相关的警告和建议（如"大型数据库解析可能需要较长时间"）
4. WHEN 运维人员点击"创建任务"按钮，THE EguiConfigWizard SHALL 调用 POST /api/tasks API 创建任务，并在 5 秒内显示创建结果
5. WHEN 任务创建成功，THE EguiConfigWizard SHALL 显示成功提示，并提供"查看任务"和"创建新任务"两个选项

### Requirement 7: 数据库启动和管理

**User Story:** 作为运维人员，我希望通过 GUI 界面启动和管理 SurrealDB 数据库，以便为任务执行提供数据存储支持

#### Acceptance Criteria

1. WHEN 运维人员访问数据库管理页面，THE EguiConfigWizard SHALL 显示 SurrealDB 配置表单，包含主机地址、端口、命名空间、数据库名、用户名和密码
2. WHEN 运维人员点击"启动数据库"按钮，THE EguiConfigWizard SHALL 在后台启动 SurrealDB 进程，并在 10 秒内显示启动结果（成功/失败、监听地址）
3. WHEN SurrealDB 启动成功，THE EguiConfigWizard SHALL 在状态栏显示绿色指示器和连接信息（如 ws://localhost:8000）
4. WHEN 运维人员点击"停止数据库"按钮，THE EguiConfigWizard SHALL 优雅关闭 SurrealDB 进程，并在 10 秒内显示停止结果
5. WHEN SurrealDB 运行时发生错误，THE EguiConfigWizard SHALL 在界面顶部显示错误横幅，包含错误信息和重启按钮
6. WHEN 运维人员点击"测试连接"按钮，THE EguiConfigWizard SHALL 尝试连接到 SurrealDB，并显示连接测试结果（成功/失败、版本信息、延迟）

### Requirement 8: 任务监控和进度显示

**User Story:** 作为运维人员，我希望实时查看任务执行状态和进度，以便及时了解任务完成情况

#### Acceptance Criteria

1. WHEN 运维人员访问任务监控页面，THE EguiConfigWizard SHALL 显示任务列表，包含任务名称、类型、状态、进度条和操作按钮
2. WHEN 任务状态为"运行中"，THE EguiConfigWizard SHALL 显示实时进度条（0-100%）和当前步骤描述（如"正在解析数据库 7999"）
3. WHEN 运维人员点击任务行，THE EguiConfigWizard SHALL 在弹窗中显示任务详情，包含开始时间、已耗时、预计剩余时间、详细日志
4. WHEN 运维人员点击"取消任务"按钮，THE EguiConfigWizard SHALL 显示确认对话框，确认后调用 POST /api/tasks/{id}/cancel API 取消任务
5. WHEN 任务完成或失败，THE EguiConfigWizard SHALL 显示完成状态（成功/失败）和结果摘要（处理记录数、生成文件数、错误信息）

### Requirement 9: 配置文件解析和生成

**User Story:** 作为运维人员，我希望通过 GUI 界面编辑和生成 DbOption.toml 配置文件，以便快速配置应用参数

#### Acceptance Criteria

1. WHEN 运维人员访问配置编辑页面，THE EguiConfigWizard SHALL 加载并解析 DbOption.toml 文件，显示所有配置项的当前值
2. WHEN 运维人员修改配置项，THE EguiConfigWizard SHALL 实时验证输入格式（端口号、路径、布尔值等），并显示验证错误
3. WHEN 运维人员点击"保存配置"按钮，THE EguiConfigWizard SHALL 将配置序列化为 TOML 格式，并写入 DbOption.toml 文件
4. WHEN 配置文件保存成功，THE EguiConfigWizard SHALL 显示成功提示，并询问是否重启相关服务以应用新配置
5. WHEN 配置文件格式错误或损坏，THE EguiConfigWizard SHALL 显示错误详情，并提供"恢复默认配置"和"手动编辑"两个选项

### Requirement 10: 批量任务创建

**User Story:** 作为运维人员，我希望批量创建多个相似任务，以便快速处理多个数据库或站点

#### Acceptance Criteria

1. WHEN 运维人员在任务创建向导中启用"批量模式"，THE EguiConfigWizard SHALL 显示批量配置选项，包含目标站点多选和参数模板
2. WHEN 运维人员选择多个站点，THE EguiConfigWizard SHALL 显示选中站点列表，并允许为每个站点单独配置参数或使用统一参数
3. WHEN 运维人员配置参数模板，THE EguiConfigWizard SHALL 支持变量替换（如 {site_name}、{db_num}），并在预览中显示每个任务的实际参数
4. WHEN 运维人员点击"批量创建"按钮，THE EguiConfigWizard SHALL 为每个站点创建独立任务，并显示创建进度（如"已创建 3/5 个任务"）
5. WHEN 批量创建完成，THE EguiConfigWizard SHALL 显示创建摘要，包含成功数量、失败数量和失败原因列表

### Requirement 11: 任务模板管理

**User Story:** 作为运维人员，我希望保存和复用任务配置模板，以便快速创建常用任务

#### Acceptance Criteria

1. WHEN 运维人员在任务创建向导中点击"保存为模板"按钮，THE EguiConfigWizard SHALL 显示模板保存对话框，要求输入模板名称和描述
2. WHEN 运维人员保存模板，THE EguiConfigWizard SHALL 将当前任务配置（不包含站点选择）保存到本地配置文件（~/.config/egui_remote_sync/task_templates.json）
3. WHEN 运维人员访问任务创建页面，THE EguiConfigWizard SHALL 在顶部显示"从模板创建"按钮和已保存的模板列表
4. WHEN 运维人员选择模板，THE EguiConfigWizard SHALL 加载模板配置，自动填充任务类型、优先级和参数，但保留站点选择为空
5. WHEN 运维人员删除模板，THE EguiConfigWizard SHALL 显示确认对话框，确认后从配置文件中移除模板

### Requirement 12: 错误处理和用户反馈

**User Story:** 作为运维人员，我希望在操作失败时获得清晰的错误提示和恢复建议，以便快速解决问题

#### Acceptance Criteria

1. WHEN API 调用失败（网络错误、超时、服务器错误），THE EguiConfigWizard SHALL 在界面顶部显示错误横幅，包含错误类型、详细信息和"重试"按钮
2. WHEN 表单验证失败，THE EguiConfigWizard SHALL 在对应输入框下方显示红色错误提示文本，并禁用"下一步"或"创建"按钮
3. WHEN 数据库启动失败，THE EguiConfigWizard SHALL 显示详细错误信息（如"端口 8000 已被占用"），并提供解决建议（如"请修改端口或停止占用进程"）
4. WHEN 任务创建成功，THE EguiConfigWizard SHALL 在界面右下角显示绿色成功提示（Toast），3 秒后自动消失
5. WHEN 发生致命错误（如配置文件损坏、数据库连接失败），THE EguiConfigWizard SHALL 显示模态对话框，包含错误详情、建议解决方案和"查看日志"按钮
