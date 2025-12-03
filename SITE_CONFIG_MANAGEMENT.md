# 站点配置管理功能文档

## 功能概述

站点配置管理功能允许用户通过 Web 界面编辑和管理 `DbOption.toml` 配置文件，避免手动编辑配置文件带来的错误。用户可以通过直观的表单界面配置项目路径、数据库连接、MQTT、服务器等参数。

## 功能特性

### ✅ 已实现功能

1. **配置读取 API** (`GET /api/site-config`)
   - 读取当前 `DbOption.toml` 的配置
   - 返回结构化的 JSON 数据

2. **配置保存 API** (`POST /api/site-config/save`)
   - 接收前端提交的配置数据
   - 更新 `DbOption.toml` 文件
   - 保留原有注释和格式

3. **配置验证 API** (`POST /api/site-config/validate`)
   - 验证配置项的有效性
   - 检查路径是否存在
   - 验证 IP、端口格式
   - 确保必填项不为空

4. **Web 配置界面**
   - 分组展示配置项（项目设置、位置和数据库、数据库连接、MQTT、服务器、模型生成、同步配置）
   - 实时输入验证
   - 友好的错误提示
   - 响应式设计

## 技术架构

### 后端实现

#### 文件结构
```
src/web_server/
├── site_config_handlers.rs  # 配置管理 API 处理器
└── mod.rs                    # 路由注册
```

#### API 端点

1. **GET /api/site-config**
   - 功能：读取当前配置
   - 响应：
     ```json
     {
       "status": "success",
       "config": {
         "project_path": "D:/AVEVA/Projects/E3D2.1",
         "included_projects": ["AvevaMarineSample", "AvevaCatalogue"],
         "project_name": "AvevaMarineSample",
         "project_code": "1516",
         "module": "DESI",
         "location": "SJZ",
         "location_dbs": [1112],
         "ip": "127.0.0.1",
         "user": "root",
         "password": "",
         "port": "3306",
         "mqtt_host": "192.168.31.58",
         "mqtt_port": 1883,
         "server_release_ip": "127.0.0.1:9099",
         "file_server_host": "http://192.168.31.58:8000/assets/archives",
         "gen_model": true,
         "gen_mesh": true,
         "gen_spatial_tree": true,
         "apply_boolean_operation": true,
         "mesh_tol_ratio": 3.0,
         "total_sync": true,
         "incr_sync": false,
         "sync_live": true
       },
       "config_file_location": "SJZ"
     }
     ```

2. **POST /api/site-config/save**
   - 功能：保存配置到文件
   - 请求体：`SiteConfig` JSON 对象
   - 响应：
     ```json
     {
       "status": "success",
       "message": "配置已保存，部分配置需要重启服务器后生效"
     }
     ```

3. **POST /api/site-config/validate**
   - 功能：验证配置有效性
   - 请求体：`SiteConfig` JSON 对象
   - 响应（成功）：
     ```json
     {
       "status": "success",
       "message": "配置验证通过"
     }
     ```
   - 响应（失败）：
     ```json
     {
       "status": "error",
       "message": "配置验证失败",
       "errors": [
         "项目路径不存在: D:/invalid/path",
         "无效的 IP 地址: 999.999.999.999",
         "MQTT 端口不能为 0"
       ]
     }
     ```

#### 核心数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    // 项目设置
    pub project_path: String,
    pub included_projects: Vec<String>,
    pub project_name: String,
    pub project_code: String,
    pub module: String,

    // 位置和数据库
    pub location: String,
    pub location_dbs: Vec<u32>,

    // 数据库连接参数
    pub ip: String,
    pub user: String,
    pub password: String,
    pub port: String,

    // MQTT 配置
    pub mqtt_host: String,
    pub mqtt_port: u16,

    // 服务器配置
    pub server_release_ip: String,
    pub file_server_host: String,

    // 模型生成配置
    pub gen_model: bool,
    pub gen_mesh: bool,
    pub gen_spatial_tree: bool,
    pub apply_boolean_operation: bool,
    pub mesh_tol_ratio: f32,

    // 同步配置
    pub total_sync: bool,
    pub incr_sync: bool,
    pub sync_live: bool,
}
```

### 前端实现

#### 文件结构
```
frontend/src/
├── components/
│   └── views/
│       └── SiteConfig.vue  # 配置管理页面组件
└── App.vue                 # 主应用（集成导航）
```

#### 组件功能

`SiteConfig.vue` 组件提供以下功能：

1. **配置加载**
   - 组件挂载时自动加载配置
   - 显示加载状态

2. **表单分组**
   - 项目设置
   - 位置和数据库
   - 数据库连接参数
   - MQTT 配置
   - 服务器配置
   - 模型生成配置
   - 同步配置

3. **数据类型处理**
   - 数组字段（`included_projects`, `location_dbs`）使用逗号分隔的字符串输入
   - 自动转换为数组格式

4. **验证功能**
   - 点击"验证配置"按钮触发验证
   - 显示验证错误的模态框

5. **保存功能**
   - 点击"保存配置"触发保存
   - 显示确认对话框
   - 保存成功后显示提示

## 使用指南

### 访问配置页面

1. 启动 Web 服务器：
   ```bash
   cargo run --bin web_server --features web_server
   ```

2. 打开浏览器访问：`http://127.0.0.1:9099`

3. 在左侧导航栏点击 **"系统" → "站点配置"**

### 编辑配置

1. **项目设置**
   - 修改项目路径、项目名称、项目代码、模块等
   - 多个项目用逗号分隔（如：`AvevaMarineSample, AvevaCatalogue`）

2. **位置和数据库**
   - 设置站点位置标识（用于 MQTT 和异地同步）
   - 配置数据库编号列表（如：`1112, 1113`）

3. **数据库连接**
   - 配置 MySQL/TiDB 的 IP、端口、用户名、密码

4. **MQTT 配置**
   - 配置 MQTT Broker 地址和端口

5. **服务器配置**
   - 配置 Web 服务器监听地址
   - 配置文件服务器地址（用于异地同步）

6. **模型生成配置**
   - 勾选是否生成模型、网格、空间树
   - 是否应用布尔运算
   - 调整网格容差比例

7. **同步配置**
   - 勾选完全同步、增量同步、实时同步选项

### 验证和保存

1. **验证配置**
   - 点击右上角"验证配置"按钮
   - 系统会检查：
     - 项目路径是否存在
     - IP 地址格式是否正确
     - 端口号是否有效
     - 必填字段是否完整
   - 如有错误，会弹出错误列表

2. **保存配置**
   - 点击"保存配置"按钮
   - 确认保存操作
   - 系统会更新 `DbOption.toml` 文件
   - **注意**：部分配置需要重启服务器后生效

## 配置项说明

### 项目设置

| 配置项 | 类型 | 说明 | 示例 |
|--------|------|------|------|
| `project_path` | String | PDMS/E3D 项目根目录 | `D:/AVEVA/Projects/E3D2.1` |
| `included_projects` | Array | 包含的项目列表 | `["AvevaMarineSample", "AvevaCatalogue"]` |
| `project_name` | String | 项目名称 | `AvevaMarineSample` |
| `project_code` | String | 项目代码（用于命名空间） | `1516` |
| `module` | String | 模块名称 | `DESI` |

### 位置和数据库

| 配置项 | 类型 | 说明 | 示例 |
|--------|------|------|------|
| `location` | String | 站点位置标识（用于 MQTT） | `SJZ` |
| `location_dbs` | Array | 该站点的数据库编号列表 | `[1112, 1113]` |

### 数据库连接

| 配置项 | 类型 | 说明 | 示例 |
|--------|------|------|------|
| `ip` | String | MySQL/TiDB IP 地址 | `127.0.0.1` |
| `user` | String | 数据库用户名 | `root` |
| `password` | String | 数据库密码 | `` |
| `port` | String | 数据库端口 | `3306` |

### MQTT 配置

| 配置项 | 类型 | 说明 | 示例 |
|--------|------|------|------|
| `mqtt_host` | String | MQTT Broker 地址 | `192.168.31.58` |
| `mqtt_port` | u16 | MQTT Broker 端口 | `1883` |

### 服务器配置

| 配置项 | 类型 | 说明 | 示例 |
|--------|------|------|------|
| `server_release_ip` | String | Web 服务器监听地址 | `127.0.0.1:9099` |
| `file_server_host` | String | 文件服务器地址（异地同步） | `http://192.168.31.58:8000/assets/archives` |

### 模型生成配置

| 配置项 | 类型 | 说明 | 示例 |
|--------|------|------|------|
| `gen_model` | bool | 是否生成模型数据 | `true` |
| `gen_mesh` | bool | 是否生成网格数据 | `true` |
| `gen_spatial_tree` | bool | 是否生成空间树 | `true` |
| `apply_boolean_operation` | bool | 是否应用布尔运算 | `true` |
| `mesh_tol_ratio` | f32 | 网格容差比例（越大越粗糙） | `3.0` |

### 同步配置

| 配置项 | 类型 | 说明 | 示例 |
|--------|------|------|------|
| `total_sync` | bool | 完全同步 | `true` |
| `incr_sync` | bool | 增量同步 | `false` |
| `sync_live` | bool | 实时同步（SurrealDB） | `true` |

## 注意事项

### 配置文件备份

在修改配置前，建议备份 `DbOption.toml`：
```bash
cp DbOption.toml DbOption.toml.backup
```

### 配置生效时机

- **立即生效**：部分运行时配置（如日志级别）
- **需要重启**：大部分配置项需要重启 Web 服务器才能生效

### 配置验证

保存前务必点击"验证配置"，确保：
1. 路径存在且可访问
2. IP、端口格式正确
3. 必填字段已填写

### 权限要求

- 用户需要对 `DbOption.toml` 文件有读写权限
- 项目路径需要有读取权限
- 数据库连接需要有相应的访问权限

## 故障排查

### 问题1：保存配置失败

**现象**：点击保存后显示"保存配置失败"

**排查步骤**：
1. 检查 `DbOption.toml` 文件是否存在
2. 检查文件权限（是否可写）
3. 检查配置项格式是否正确
4. 查看服务器日志获取详细错误信息

### 问题2：验证失败

**现象**：验证配置时显示错误列表

**解决方法**：
1. 根据错误提示逐项修正
2. 确保路径使用正确的分隔符（Windows 使用 `\` 或 `/`）
3. 确保 IP 地址格式正确（如 `192.168.1.1` 或 `localhost`）
4. 确保端口号在有效范围（1-65535）

### 问题3：配置未生效

**现象**：保存配置后，系统仍使用旧配置

**解决方法**：
1. 重启 Web 服务器：
   ```bash
   # 停止当前服务
   Ctrl+C

   # 重新启动
   cargo run --bin web_server --features web_server
   ```
2. 检查 `DbOption.toml` 文件内容是否已更新
3. 检查是否有其他进程占用配置文件

## 开发说明

### 添加新配置项

如需添加新的配置项，需要修改以下文件：

1. **后端**：`src/web_server/site_config_handlers.rs`
   - 在 `SiteConfig` 结构体中添加新字段
   - 在 `read_config()` 函数中读取配置
   - 在 `write_config()` 函数中写入配置
   - 在 `validate_site_config()` 中添加验证逻辑

2. **前端**：`frontend/src/components/views/SiteConfig.vue`
   - 在 `config` 响应式对象中添加字段
   - 在模板中添加对应的表单控件
   - 在 `validateConfig()` 中添加前端验证

### 测试

#### 后端测试
```bash
cargo check --bin web_server --features web_server
cargo build --bin web_server --features web_server
```

#### 前端测试
```bash
cd frontend
npm run build
```

#### 功能测试
1. 启动服务器
2. 访问配置页面
3. 修改配置并保存
4. 检查 `DbOption.toml` 是否正确更新
5. 重启服务器验证配置生效

## 相关文档

- [MQTT 主从节点角色管理](MQTT_MASTER_CLIENT_ROLE.md)
- [MQTT 订阅控制文档](MQTT_SUBSCRIPTION_CONTROL.md)
- [增量更新实现指南](docs/INCREMENT_DETECTION_FLOWCHART.md)
- [异地同步开发指南](docs/REMOTE_SYNC_DEVELOPMENT_GUIDE.md)

## 更新历史

- **2025-01-20**: 初始版本，支持基本配置管理
  - 实现配置读取、保存、验证 API
  - 实现 Web 配置界面
  - 集成到主界面导航
  - 编译测试通过
