# Implementation Plan

- [ ] 1. 实现任务创建向导核心组件
  - 创建 src/gui/pages/task_creation.rs 和 src/gui/components/task_wizard.rs 文件
  - 实现 TaskCreationWizard 结构体，包含 4 个步骤（BasicInfo/SelectSite/Parameters/Preview）
  - 实现步骤指示器 UI，显示当前步骤和完成状态
  - 实现步骤导航逻辑（上一步/下一步/验证）
  - 实现 TaskCreationFormData 数据结构，包含任务名称、类型、优先级、描述和参数
  - 实现 TaskType 枚举（DataParsing/ModelGeneration/SpatialTreeGeneration/FullSync/IncrementalSync）
  - 实现 TaskPriority 枚举（Low/Normal/High/Urgent）
  - _Requirements: 1.1-1.5_

- [ ] 2. 实现基础信息配置步骤
  - 实现 render_basic_info 方法，显示任务名称、类型、描述和优先级输入框
  - 实现任务类型下拉选择器，显示图标和描述
  - 实现优先级单选按钮组
  - 实现任务名称唯一性验证
  - 实现表单验证逻辑，显示错误提示
  - _Requirements: 1.1-1.5_

- [ ] 3. 实现站点选择步骤
  - 实现 render_site_selection 方法，显示可用站点列表
  - 实现单选模式（单个站点）和批量模式（多个站点）切换
  - 实现站点列表表格，显示站点名称、环境、地区和状态
  - 实现站点选择逻辑（单选/多选）
  - 实现"测试连接"按钮，调用 API 测试站点连接
  - 实现站点详情显示（HTTP 地址、数据库编号、SurrealDB 配置）
  - _Requirements: 5.1-5.5_

- [ ] 4. 实现数据解析任务参数配置
  - 实现 render_data_parsing_params 方法
  - 实现解析模式选择器（全部解析/指定数据库编号/指定参考号）
  - 实现数据库编号输入框，支持逗号分隔的多个编号
  - 实现参考号输入框
  - 实现参数验证逻辑（数据库编号格式、参考号格式）
  - 实现 TaskParameters::DataParsing 数据结构
  - _Requirements: 2.1-2.5_

- [ ] 5. 实现模型生成任务参数配置
  - 实现 render_model_generation_params 方法
  - 实现生成选项复选框（3D 模型/网格/空间树/布尔运算）
  - 实现网格容差比例滑块（0.001-1.0）
  - 实现最大并发数输入框（1-32）
  - 实现并行处理开关和高级选项
  - 实现 TaskParameters::ModelGeneration 数据结构
  - _Requirements: 3.1-3.5_

- [ ] 6. 实现空间树生成任务参数配置
  - 实现 render_spatial_tree_params 方法
  - 实现树深度滑块（1-10）
  - 实现节点容量输入框（10-1000）
  - 实现空间索引类型下拉选择器（R-Tree/Quad-Tree/Oct-Tree）
  - 实现 TaskParameters::SpatialTreeGeneration 数据结构
  - _Requirements: 4.1-4.5_

- [ ] 7. 实现任务预览和确认步骤
  - 实现 render_preview 方法，显示任务配置摘要
  - 显示任务名称、类型、优先级、描述和目标站点
  - 显示任务参数摘要（根据任务类型）
  - 显示资源需求预估（内存、磁盘空间、预计耗时）
  - 显示注意事项和警告信息
  - 实现"创建任务"按钮，调用 POST /api/tasks API
  - 实现任务创建成功/失败提示
  - _Requirements: 6.1-6.5_

- [ ] 8. 实现批量任务创建功能
  - 实现批量模式开关和多站点选择
  - 实现参数模板配置，支持变量替换（{site_name}、{db_num}）
  - 实现批量任务预览，显示每个任务的实际参数
  - 实现批量创建逻辑，为每个站点创建独立任务
  - 显示批量创建进度（已创建 X/Y 个任务）
  - 显示批量创建摘要（成功数量、失败数量、失败原因）
  - _Requirements: 10.1-10.5_

- [ ] 9. 实现任务模板管理功能
  - 创建 TaskTemplate 数据结构
  - 实现"保存为模板"功能，保存当前任务配置
  - 实现模板保存对话框，输入模板名称和描述
  - 实现模板列表显示和选择
  - 实现"从模板创建"功能，加载模板配置
  - 实现模板删除功能
  - 实现模板持久化到配置文件（~/.config/egui_remote_sync/task_templates.json）
  - _Requirements: 11.1-11.5_

- [ ] 10. 实现数据库管理页面
  - 创建 src/gui/pages/database_manage.rs 文件
  - 实现 DatabaseManagePage 结构体
  - 实现 SurrealDBConfig 数据结构（主机、端口、命名空间、数据库、用户名、密码、数据路径）
  - 实现数据库配置表单，支持所有配置项编辑
  - 实现"启动数据库"按钮，使用 std::process::Command 启动 SurrealDB 进程
  - 实现"停止数据库"按钮，优雅关闭 SurrealDB 进程
  - 实现"测试连接"按钮，测试 SurrealDB 连接
  - 实现数据库状态显示（已停止/启动中/运行中/停止中/错误）
  - 实现数据库日志输出显示
  - 实现配置保存到 DbOption.toml
  - _Requirements: 7.1-7.6_

- [ ] 11. 实现任务监控页面
  - 创建 src/gui/pages/task_monitor.rs 文件
  - 实现 TaskMonitorPage 结构体
  - 实现 TaskInfo 数据结构（ID、名称、类型、状态、进度、当前步骤、时间戳）
  - 实现任务列表表格，显示所有任务
  - 实现任务状态显示（等待中/运行中/完成/失败/已取消）
  - 实现任务进度条，显示百分比和当前步骤
  - 实现"取消任务"按钮，调用 POST /api/tasks/{id}/cancel API
  - 实现"删除任务"按钮，调用 DELETE /api/tasks/{id} API
  - 实现任务详情对话框，显示完整任务信息
  - 实现自动刷新功能（每 5 秒）
  - _Requirements: 8.1-8.5_

- [ ] 12. 实现配置编辑页面
  - 创建 src/gui/pages/config_editor.rs 文件
  - 实现 ConfigEditorPage 结构体
  - 实现 DbOptionConfig 数据结构，包含所有配置项
  - 实现表单编辑模式，使用 Grid 布局显示所有配置项
  - 实现文本编辑模式，使用 TextEdit 显示 TOML 文本
  - 实现编辑模式切换（表单模式/文本模式）
  - 实现配置验证逻辑（端口号、路径、布尔值等）
  - 实现"保存配置"按钮，序列化为 TOML 并写入文件
  - 实现"重新加载"按钮，从文件加载配置
  - 实现"恢复默认"按钮，重置为默认配置
  - 实现验证错误显示
  - _Requirements: 9.1-9.5_

- [ ] 13. 实现 API 客户端扩展
  - 在 src/gui/api_client.rs 中添加任务相关 API 方法
  - 实现 create_task 方法（POST /api/tasks）
  - 实现 get_tasks 方法（GET /api/tasks）
  - 实现 get_task 方法（GET /api/tasks/{id}）
  - 实现 cancel_task 方法（POST /api/tasks/{id}/cancel）
  - 实现 delete_task 方法（DELETE /api/tasks/{id}）
  - 实现 get_task_templates 方法（GET /api/task-templates）
  - 实现 create_task_template 方法（POST /api/task-templates）
  - 实现 delete_task_template 方法（DELETE /api/task-templates/{id}）
  - _Requirements: 1.1-12.5_

- [ ] 14. 集成到主应用
  - 在 src/gui/app.rs 中添加新页面枚举值（TaskCreation/TaskMonitor/DatabaseManage/ConfigEditor）
  - 在导航栏中添加新页面入口，分组为"数据处理"和"系统管理"
  - 实现页面路由逻辑，根据当前页面渲染对应组件
  - 更新 AppState，添加任务列表和模板列表
  - 实现页面间数据共享和状态同步
  - _Requirements: 1.1-12.5_

- [ ] 15. 实现错误处理和用户反馈
  - 创建 ConfigWizardError 错误类型
  - 实现 API 调用错误处理，显示错误横幅
  - 实现表单验证错误处理，显示红色错误提示
  - 实现数据库启动失败错误处理，显示详细错误信息和解决建议
  - 实现成功提示（Toast），3 秒后自动消失
  - 实现致命错误对话框，包含错误详情和"查看日志"按钮
  - 集成 ToastManager 到所有页面
  - _Requirements: 12.1-12.5_

- [ ] 16. 实现配置持久化
  - 实现任务模板保存到 ~/.config/egui_remote_sync/task_templates.json
  - 实现任务模板从配置文件加载
  - 实现 DbOption.toml 配置文件读写
  - 实现配置文件格式验证和错误恢复
  - 实现配置文件备份功能
  - _Requirements: 9.1-9.5, 11.1-11.5_

- [ ]* 17. 编写单元测试
  - 编写任务参数验证逻辑测试
  - 编写配置文件解析和序列化测试
  - 编写任务模板保存和加载测试
  - 编写表单验证逻辑测试
  - _Requirements: 1.1-12.5_

- [ ]* 18. 编写集成测试
  - 编写任务创建流程端到端测试
  - 编写数据库启动和停止测试
  - 编写 API 调用集成测试
  - 编写批量任务创建测试
  - _Requirements: 1.1-12.5_

- [ ]* 19. 编写用户文档
  - 编写任务创建向导使用指南
  - 编写数据库管理使用指南
  - 编写配置编辑使用指南
  - 编写任务模板管理使用指南
  - 编写常见问题和故障排除指南
  - _Requirements: 1.1-12.5_
