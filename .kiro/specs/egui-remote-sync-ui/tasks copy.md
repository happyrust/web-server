# Implementation Plan

- [ ] 1. 项目初始化和基础架构
  - 创建 Cargo 项目，配置依赖（egui 0.33, eframe, re_ui 0.27.2, reqwest, tokio, serde_json, rusqlite 等）
  - 创建目录结构（src/gui/, src/gui/pages/, src/gui/components/, src/gui/canvas/）
  - 实现主应用入口（src/bin/egui_remote_sync.rs）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [ ] 2. 核心应用框架和状态管理
  - [ ] 2.1 实现 EguiRemoteSyncApp 主应用结构
    - 初始化 re_ui 和主题
    - 实现 eframe::App trait
    - 实现顶部栏和导航面板渲染
    - _Requirements: 1.1, 1.2, 16.1, 16.6_
  
  - [ ] 2.2 实现 AppState 全局状态管理
    - 定义状态结构（environments, sites, sync_tasks, logs, server_status 等）
    - 实现状态加载和更新方法
    - _Requirements: 1.5, 5.1_
  
  - [ ] 2.3 实现 ApiClient HTTP 客户端
    - 实现环境管理 API 方法（get_environments, create_environment, update_environment, delete_environment, activate_environment）
    - 实现站点管理 API 方法（get_sites, create_site, test_site_connection）
    - 实现同步控制 API 方法（start_sync, stop_sync, pause_sync, resume_sync, clear_queue, get_sync_status）
    - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 3. 环境和站点管理页面
  - [ ] 3.1 实现 EnvironmentListPage 环境列表页面
    - 使用 re_ui 表格显示环境列表
    - 实现添加/编辑/删除环境功能
    - 实现激活环境功能
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_
  
  - [ ] 3.2 实现 EnvironmentForm 环境配置表单
    - 实现表单字段（名称、MQTT 主机、MQTT 端口、文件服务器地址、地区标识、数据库编号）
    - 实现表单验证（端口范围、URL 格式、数据库编号格式）
    - 实现文件选择器集成
    - _Requirements: 2.2, 2.3_
  
  - [ ] 3.3 实现站点配置管理
    - 实现站点列表显示
    - 实现添加/编辑/删除站点功能
    - 实现测试连接功能
    - 实现查看元数据功能
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 4. 拓扑画布编辑器
  - [ ] 4.1 实现 TopologyCanvas 画布核心
    - 实现画布渲染（网格背景、节点、连线）
    - 实现鼠标交互（点击、拖拽、缩放、平移）
    - 实现节点创建（环境节点、站点节点）
    - _Requirements: 4.1, 4.2, 4.3_
  
  - [ ] 4.2 实现拓扑节点和连线
    - 定义 TopologyNode 和 TopologyEdge 数据结构
    - 实现环境节点渲染（矩形，蓝色边框）
    - 实现站点节点渲染（圆形，绿色边框）
    - 实现连线渲染（箭头）
    - _Requirements: 4.2, 4.3, 4.4_
  
  - [ ] 4.3 实现拓扑编辑功能
    - 实现节点选择和配置面板
    - 实现节点连接功能
    - 实现自动布局算法（层次布局）
    - 实现拓扑验证（环境必须有 MQTT 配置、站点必须关联环境）
    - _Requirements: 4.5, 4.6, 4.7_
  
  - [ ] 4.4 实现拓扑导入导出
    - 实现保存拓扑到后端 API
    - 实现导出为 JSON 文件
    - 实现从 JSON 文件导入
    - _Requirements: 4.7, 4.8, 4.9_

- [ ] 5. 实时监控面板
  - [ ] 5.1 实现 MonitorDashboardPage 监控面板
    - 实现状态卡片显示（运行状态、MQTT 连接、队列大小、活跃任务）
    - 实现任务列表表格（文件名、源环境、目标站点、状态、进度）
    - 实现自动刷新（每 5 秒）
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_
  
  - [ ] 5.2 实现任务详情查看
    - 实现任务详情对话框
    - 显示完整的任务信息（文件路径、大小、记录数、耗时、错误信息）
    - _Requirements: 5.5_

- [ ] 6. 运维操作工具栏
  - [ ] 6.1 实现运维操作按钮
    - 实现启动同步按钮
    - 实现停止同步按钮（带确认对话框）
    - 实现暂停同步按钮
    - 实现恢复同步按钮
    - 实现清空队列按钮（带二次确认）
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 7. 日志查询页面
  - [ ] 7.1 实现 LogQueryPage 日志查询页面
    - 实现筛选表单（环境、站点、状态、时间范围）
    - 实现日志列表表格
    - 实现分页控件
    - _Requirements: 7.1, 7.2, 7.3_
  
  - [ ] 7.2 实现日志详情和导出
    - 实现日志详情对话框
    - 实现导出 CSV 功能
    - _Requirements: 7.4, 7.5_

- [ ] 8. Web Server 管理页面
  - [ ] 8.1 实现 WebServerPage 服务器管理页面
    - 实现服务器状态显示
    - 实现配置表单（监听地址、监听端口、数据库路径、静态文件目录）
    - 实现启动/停止服务器功能
    - 实现日志输出显示
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 9. 解析任务管理页面
  - [ ] 9.1 实现 ParseTaskPage 解析任务页面
    - 实现任务列表表格（任务名称、数据库路径、状态、进度）
    - 实现新建任务对话框
    - 实现任务操作（开始、取消、删除）
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5, 13.6, 13.7_
  
  - [ ] 9.2 实现 ParseTaskForm 解析任务表单
    - 实现表单字段（任务名称、数据库路径、输出目录）
    - 实现表单验证（数据库文件存在、输出目录有效）
    - 实现文件选择器集成
    - _Requirements: 13.2, 13.3_
  
  - [ ] 9.3 实现解析任务 API 集成
    - 实现 get_parse_tasks API 方法
    - 实现 create_parse_task API 方法
    - 实现 start_parse_task API 方法
    - 实现 cancel_parse_task API 方法
    - 实现 delete_parse_task API 方法
    - _Requirements: 13.3, 13.4, 13.5, 13.6, 13.7_

- [ ] 10. 模型生成配置页面
  - [ ] 10.1 实现 ModelGenPage 模型生成页面
    - 实现配置列表（左侧面板）
    - 实现配置详情和生成任务列表（右侧面板）
    - 实现新建/编辑配置对话框
    - _Requirements: 14.1, 14.2, 14.7_
  
  - [ ] 10.2 实现 ModelGenConfigForm 模型生成配置表单
    - 实现表单字段（配置名称、数据库路径、输出格式、LOD 级别、包含材质、压缩输出）
    - 实现表单验证（至少选择一个 LOD 级别）
    - 实现文件选择器集成
    - _Requirements: 14.2, 14.3_
  
  - [ ] 10.3 实现模型生成功能
    - 实现开始生成按钮
    - 实现生成任务列表显示
    - 实现打开输出目录功能
    - _Requirements: 14.4, 14.5, 14.6_
  
  - [ ] 10.4 实现模型生成 API 集成
    - 实现 get_model_gen_configs API 方法
    - 实现 save_model_gen_config API 方法
    - 实现 delete_model_gen_config API 方法
    - 实现 start_model_generation API 方法
    - 实现 get_generation_tasks API 方法
    - _Requirements: 14.3, 14.4, 14.8_

- [ ] 11. 一键部署和启动页面
  - [ ] 11.1 实现 QuickDeployPage 一键部署页面
    - 实现部署状态和服务器状态卡片
    - 实现服务器配置表单（监听地址、监听端口、数据库路径、静态文件目录、启用 CORS、日志级别）
    - 实现一键部署并启动按钮
    - _Requirements: 15.1, 15.2, 15.3_
  
  - [ ] 11.2 实现服务器控制功能
    - 实现启动服务器功能（检查配置、写入 DbOption.toml、启动 Axum）
    - 实现停止服务器功能（优雅关闭）
    - 实现重启服务器功能
    - 实现打开浏览器功能
    - _Requirements: 15.4, 15.5, 15.6, 15.7_
  
  - [ ] 11.3 实现日志输出和错误处理
    - 实现实时日志输出显示
    - 实现错误信息显示和重试功能
    - 实现保存配置功能
    - _Requirements: 15.8, 15.9, 15.10_

- [ ] 12. 导航和页面组织
  - [ ] 12.1 实现导航分组
    - 实现异地协同分组（环境列表、拓扑配置、实时监控、日志查询）
    - 实现数据处理分组（解析任务、模型生成）
    - 实现系统管理分组（一键部署、服务器管理、设置）
    - _Requirements: 16.1, 16.2, 16.3, 16.4_
  
  - [ ] 12.2 实现导航交互
    - 实现分组展开/折叠功能
    - 实现页面链接高亮显示
    - 实现导航状态持久化
    - _Requirements: 16.5, 16.6_

- [ ] 13. 通用组件和工具
  - [ ] 13.1 实现 ToastManager 提示管理器
    - 实现 Toast 数据结构（成功、错误、警告、信息）
    - 实现 Toast 渲染（右上角显示，3 秒后自动消失）
    - _Requirements: 12.4_
  
  - [ ] 13.2 实现确认对话框组件
    - 实现通用确认对话框
    - 支持自定义标题、描述和按钮文本
    - _Requirements: 6.2, 6.5, 12.5_
  
  - [ ] 13.3 实现错误处理
    - 定义 AppError 错误类型
    - 实现 API 调用错误处理
    - 实现表单验证错误显示
    - _Requirements: 12.1, 12.2, 12.3, 12.5_

- [ ] 14. 配置持久化和加载
  - [ ] 14.1 实现配置文件管理
    - 实现窗口布局保存和加载（~/.config/egui_remote_sync/layout.json）
    - 实现当前页面保存和恢复
    - 实现导航状态保存和恢复
    - _Requirements: 10.1, 10.2, 10.4_
  
  - [ ] 14.2 实现数据库配置管理
    - 实现从 SQLite 数据库加载环境和站点配置
    - 实现配置变更自动保存
    - _Requirements: 10.1, 10.3_

- [ ] 15. 主题和样式
  - [ ] 15.1 实现 re_ui 主题集成
    - 应用 re_ui 默认主题
    - 实现自定义颜色配置
    - _Requirements: 11.1, 11.2_
  
  - [ ] 15.2 实现主题切换
    - 实现浅色/深色主题切换
    - 实现字体大小调整
    - 实现主题配置持久化
    - _Requirements: 11.2, 11.3, 11.4, 11.5_

- [ ] 16. 构建和打包
  - [ ] 16.1 配置 Cargo.toml
    - 添加所有依赖（egui 0.33, eframe, re_ui 0.27.2, reqwest, tokio, serde_json, rusqlite, chrono, anyhow, thiserror, tracing, rfd, csv, toml, uuid）
    - 配置 release 优化（opt-level = 3, lto = true, codegen-units = 1, strip = true）
  
  - [ ] 16.2 跨平台构建
    - 配置 Windows 构建（x86_64-pc-windows-gnu）
    - 配置 macOS 构建（x86_64-apple-darwin）
    - 配置 Linux 构建（x86_64-unknown-linux-gnu）
  
  - [ ] 16.3 打包分发
    - Windows: 生成 .exe 可执行文件
    - macOS: 打包为 .app 应用包
    - Linux: 生成 ELF 可执行文件或 AppImage

- [ ] 17. 集成测试和验证
  - [ ] 17.1 测试环境和站点管理
    - 测试创建、编辑、删除环境
    - 测试创建、编辑、删除站点
    - 测试激活环境
    - 测试连接测试功能
  
  - [ ] 17.2 测试拓扑画布
    - 测试节点创建和拖拽
    - 测试节点连接
    - 测试自动布局
    - 测试导入导出 JSON
  
  - [ ] 17.3 测试监控和日志
    - 测试实时监控刷新
    - 测试日志查询和过滤
    - 测试日志导出
  
  - [ ] 17.4 测试解析和模型生成
    - 测试创建解析任务
    - 测试启动和取消解析任务
    - 测试创建模型生成配置
    - 测试开始生成和查看输出
  
  - [ ] 17.5 测试一键部署
    - 测试配置服务器参数
    - 测试启动和停止服务器
    - 测试日志输出显示
    - 测试错误处理和重试

- [ ] 18. 文档和用户指南
  - [ ] 18.1 编写用户手册
    - 编写安装和启动指南
    - 编写功能使用说明
    - 编写常见问题解答
  
  - [ ] 18.2 编写开发文档
    - 编写架构设计文档
    - 编写 API 集成文档
    - 编写构建和部署文档
