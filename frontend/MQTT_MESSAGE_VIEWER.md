# MQTT 消息记录查看功能

## 功能说明

新增了 MQTT 消息记录查看功能，用于查看系统发送的所有 MQTT 增量同步消息历史。

## 访问方式

1. 启动前端界面
2. 点击左侧导航栏的 **"MQTT 消息"** 按钮
3. 或在代码中设置 `currentView = 'mqtt'`

## 主要功能

### 1. 消息列表展示
- 以卡片形式展示每条 MQTT 消息
- 显示发送时间（智能相对时间格式）
- 区分完全同步 vs 增量同步（颜色标识）
- 显示位置信息（location）和数据库编号（db_num）

### 2. 详细信息
每条消息包含：
- **会话范围**：session_range（如 "1162..=1163"）
- **变更统计**：新增/修改/删除的元素数量
- **文件列表**：file_names 和对应的 hash 值
- **服务器地址**：file_server_host

### 3. 筛选功能
- **位置筛选**：按 location 字段筛选（如 bj、sjz 等）
- **同步类型筛选**：完全同步 / 增量同步
- **文件名搜索**：支持文件名模糊搜索

### 4. 分页功能
- 默认每页显示 20 条记录
- 底部分页控件支持快速跳转
- 显示总记录数和当前页范围

### 5. 展开/收起
- 点击消息卡片右上角按钮可展开/收起文件详情
- 收起时显示前 2 个文件名预览

## 技术实现

### 前端组件
- **组件位置**：`frontend/src/components/views/MqttMessageViewer.vue`
- **路由集成**：已集成到 `App.vue` 主应用中
- **样式框架**：DaisyUI + Tailwind CSS

### API 调用
- **接口**：`GET /api/incremental/history?page={page}&page_size={pageSize}`
- **后端实现**：`src/web_server/incremental_update_handlers.rs::get_sync_history_paged()`
- **数据源**：SurrealDB 的 `e3d_sync` 表

### 数据字段
```json
{
  "file_names": ["file1.cba", "file2.cba"],
  "file_hashes": ["hash1...", "hash2..."],
  "timestamp": "2025-11-20T10:30:00Z",
  "location": "bj",
  "file_server_host": "http://192.168.1.10:3000",
  "session_range": "1162..=1163",
  "total_added": 150,
  "total_modified": 45,
  "total_deleted": 5,
  "is_full_sync": false,
  "db_num": 1112,
  "file_count": 2
}
```

## 使用场景

1. **故障排查**：查看历史 MQTT 消息，确认同步是否正常发送
2. **审计日志**：追踪系统何时、向哪里发送了增量同步消息
3. **数据分析**：分析增量同步的频率、大小和内容
4. **运维监控**：监控各个位置的同步活动

## 相关文件

- 前端组件：[frontend/src/components/views/MqttMessageViewer.vue](src/components/views/MqttMessageViewer.vue)
- 主应用集成：[frontend/src/App.vue](src/App.vue)
- API 方法：[frontend/src/composables/useApi.js](src/composables/useApi.js) 的 `loadSyncHistory()`
- 后端处理器：[src/web_server/incremental_update_handlers.rs](../../src/web_server/incremental_update_handlers.rs)
- 数据模型：[src/mqtt_service/mod.rs](../../src/mqtt_service/mod.rs) 的 `SyncE3dFileMsg`

## 开发说明

如需修改或扩展功能：

1. **添加新筛选条件**：修改 `MqttMessageViewer.vue` 中的 `filters` 对象和 `filteredMessages` 计算属性
2. **修改显示字段**：调整组件模板中的数据绑定
3. **调整分页大小**：修改 `pagination.pageSize` 默认值
4. **自定义样式**：使用 Tailwind/DaisyUI 类或添加 scoped styles

## 维护日志

- 2025-11-20：初始版本创建
  - 实现基础消息列表展示
  - 添加筛选和搜索功能
  - 集成分页支持
  - 修复所有文本编码问题
