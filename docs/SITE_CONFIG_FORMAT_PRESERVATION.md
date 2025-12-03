# 站点配置格式保留说明

## 概述

站点配置管理系统已改进，现在可以在修改配置时**完整保留原有的 TOML 文件格式**，包括：

- ✅ 注释（包括行首注释和行内注释）
- ✅ 空行
- ✅ 缩进
- ✅ 引号风格（单引号/双引号）
- ✅ 数组格式
- ✅ 未修改的配置项

## 改进前后对比

### 改进前 ❌

修改配置后，原有格式会被破坏：

```toml
# 原始文件
# 项目设置
project_path = "/old/path"
included_projects = ["Project1", "Project2"]

mqtt_host = "localhost"  # MQTT 服务器
mqtt_port = 1883
```

保存后变成：

```toml
# 原始文件
# 项目设置
project_path = "/new/path"
included_projects = ["Project1", "Project2", "Project3"]
mqtt_host = "new.host.com"
mqtt_port = 8883
```

**问题**：
- 丢失了行内注释 `# MQTT 服务器`
- 数组格式可能变成 Rust 风格 `["Project1", "Project2", "Project3"]`
- 缩进可能丢失

### 改进后 ✅

```toml
# 原始文件
# 项目设置
project_path = "/new/path"
included_projects = ["Project1", "Project2", "Project3"]

mqtt_host = "new.host.com"  # MQTT 服务器
mqtt_port = 8883
```

**优势**：
- ✅ 保留所有注释
- ✅ 保留空行和缩进
- ✅ 保留引号风格（`project_code` 使用单引号）
- ✅ 数组格式标准化为 TOML 风格

## 技术实现

### 核心函数

#### 1. `write_config` - 主写入函数

```rust
fn write_config(config: &SiteConfig) -> anyhow::Result<()>
```

**特性**：
- 逐行解析原始文件
- 识别注释、空行、键值对
- 提取原始缩进和行内注释
- 只替换需要更新的配置项

#### 2. `format_config_line` - 格式化配置行

```rust
fn format_config_line(
    indent: &str,
    key: &str,
    value: &str,
    inline_comment: Option<&str>
) -> String
```

**功能**：
- 保留原始缩进
- 附加行内注释（如果存在）
- 生成格式一致的配置行

#### 3. `format_toml_array` - 数组格式化

```rust
fn format_toml_array(arr: &[String]) -> String
fn format_toml_array_u32(arr: &[u32]) -> String
```

**输出示例**：
```toml
included_projects = ["Project1", "Project2", "Project3"]
location_dbs = [1, 2, 3]
```

## 使用示例

### API 调用

```bash
# 读取当前配置
curl http://localhost:8080/api/site-config

# 保存配置
curl -X POST http://localhost:8080/api/site-config/save \
  -H "Content-Type: application/json" \
  -d '{
    "project_path": "/new/path",
    "project_name": "NewProject",
    "location": "Beijing",
    ...
  }'

# 验证配置
curl -X POST http://localhost:8080/api/site-config/validate \
  -H "Content-Type: application/json" \
  -d '{...}'
```

### 前端集成

在 Vue 组件中：

```javascript
// 读取配置
const response = await fetch('/api/site-config');
const data = await response.json();
const config = data.config;

// 修改配置
config.location = 'Shanghai';
config.mqtt_port = 8883;

// 保存配置
await fetch('/api/site-config/save', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(config)
});
```

## 配置项列表

以下配置项支持通过 API 修改：

### 项目设置
- `project_path` - 项目路径
- `included_projects` - 包含的项目列表
- `project_name` - 项目名称
- `project_code` - 项目代码（单引号）
- `module` - 模块名称

### 位置和数据库
- `location` - 站点位置
- `location_dbs` - 位置数据库列表

### 数据库连接
- `ip` - 数据库 IP
- `user` - 用户名
- `password` - 密码
- `port` - 端口

### MQTT 配置
- `mqtt_host` - MQTT 服务器地址
- `mqtt_port` - MQTT 端口

### 服务器配置
- `server_release_ip` - 服务器发布地址
- `file_server_host` - 文件服务器地址

### 模型生成
- `gen_model` - 是否生成模型
- `gen_mesh` - 是否生成网格
- `gen_spatial_tree` - 是否生成空间树
- `apply_boolean_operation` - 是否应用布尔运算
- `mesh_tol_ratio` - 网格容差比例

### 同步配置
- `total_sync` - 完全同步
- `incr_sync` - 增量同步
- `sync_live` - 实时同步

## 注意事项

### 1. 配置生效时机

⚠️ **重要**：配置修改后**需要重启服务器**才能生效。

原因：
- 配置通过 `aios_core::get_db_option()` 在启动时加载
- 使用全局静态变量缓存
- 当前不支持热重载

### 2. 文件备份

建议在修改前备份 `DbOption.toml`：

```bash
cp DbOption.toml DbOption.toml.backup
```

### 3. 格式限制

- 数组元素之间用 `, ` 分隔（逗号后有空格）
- 字符串使用双引号，除了 `project_code` 使用单引号
- 布尔值小写：`true`/`false`
- 数字不带引号

## 测试

运行单元测试：

```bash
cargo test --lib web_server::site_config_handlers::tests --features web_server
```

测试覆盖：
- ✅ 数组格式化
- ✅ 配置行格式化
- ✅ 缩进保留
- ✅ 注释保留

## 未来改进

### 计划中的功能

1. **配置热重[object Object]n   - 添加 `/api/site-config/reload` 端点
   - 重新加载配置到内存
   - 通知相关服务更新

2. **配置分类** 📋
   - 区分可热更新和需重启的配置
   - 提供更明确的用户提示

3. **配置验证增强** ✅
   - 路径存在性检查
   - 数据库连接测试
   - MQTT 连接测试

4. **配置历史** 📜
   - 记录配置变更历史
   - 支持回滚到之前的版本

## 相关文件

- `src/web_server/site_config_handlers.rs` - 配置处理器实现
- `src/web_server/mod.rs` - 路由注册
- `DbOption.toml` - 配置文件
- `src/options.rs` - 配置结构定义

## 参考链接

- [TOML 规范](https://toml.io/)
- [Rust Serde TOML](https://docs.rs/toml/)

