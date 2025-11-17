# egui 配置向导快速入门

## 快速启动

### 构建和运行

```bash
# 构建 egui 应用
cargo build --bin egui_remote_sync --features gui

# 运行 egui 应用
cargo run --bin egui_remote_sync --features gui
```

## 功能入口

### 任务管理

1. **任务创建** - 左侧导航栏 → 任务管理 → 任务创建
2. **任务监控** - 左侧导航栏 → 任务管理 → 任务监控

### 系统管理

1. **数据库管理** - 左侧导航栏 → 系统管理 → 数据库管理
2. **配置编辑** - 左侧导航栏 → 系统管理 → 配置编辑

## 快速操作

### 创建一个数据解析任务

1. 打开"任务创建"页面
2. 输入任务名称: "解析数据库 7999"
3. 选择任务类型: "数据解析任务"
4. 点击"下一步"
5. 选择目标站点
6. 点击"下一步"
7. 选择解析模式: "指定数据库编号"
8. 输入数据库编号: "7999"
9. 点击"下一步"
10. 查看预览，点击"创建任务"

### 使用任务模板

1. 创建任务时配置好参数
2. 点击"保存为模板"
3. 下次创建任务时点击"从模板创建"
4. 选择保存的模板
5. 修改站点选择
6. 创建任务

### 启动 SurrealDB

1. 打开"数据库管理"页面
2. 配置数据路径（默认 ./data/surreal）
3. 点击"启动数据库"
4. 查看日志确认启动成功

### 编辑配置文件

1. 打开"配置编辑"页面
2. 选择"表单模式"或"文本模式"
3. 修改配置项
4. 点击"保存配置"

## 代码示例

### 扩展任务类型

在 `src/gui/components/task_wizard.rs` 中添加新的任务类型：

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    DataParsing,
    ModelGeneration,
    SpatialTreeGeneration,
    FullSync,
    IncrementalSync,
    // 添加新类型
    CustomTask,
}
```

### 添加新的任务参数

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskParameters {
    // 现有参数...
    
    // 添加新参数
    CustomTask {
        param1: String,
        param2: u32,
    },
}
```

### 实现参数渲染

在 `TaskCreationWizard::render_parameters` 中添加：

```rust
fn render_parameters(&mut self, ui: &mut egui::Ui) {
    match self.form_data.task_type {
        // 现有类型...
        
        TaskType::CustomTask => self.render_custom_task_params(ui),
    }
}

fn render_custom_task_params(&mut self, ui: &mut egui::Ui) {
    if let TaskParameters::CustomTask { param1, param2 } = &mut self.form_data.parameters {
        egui::Grid::new("custom_task_params")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("参数 1:");
                ui.text_edit_singleline(param1);
                ui.end_row();
                
                ui.label("参数 2:");
                ui.add(egui::DragValue::new(param2));
                ui.end_row();
            });
    } else {
        self.form_data.parameters = TaskParameters::CustomTask {
            param1: String::new(),
            param2: 0,
        };
    }
}
```

## 文件结构

```
src/gui/
├── components/
│   └── task_wizard.rs          # 任务创建向导组件
├── pages/
│   ├── task_creation.rs        # 任务创建页面
│   ├── task_monitor.rs         # 任务监控页面
│   ├── database_manage.rs      # 数据库管理页面
│   └── config_editor.rs        # 配置编辑页面
├── api_client.rs               # API 客户端（含任务管理方法）
└── app.rs                      # 主应用（集成所有页面）
```

## 配置文件

### 任务模板

位置: `~/.config/egui_remote_sync/task_templates.json`

格式:
```json
[
  {
    "name": "数据解析模板",
    "description": "用于解析 PDMS 数据库",
    "task_type": "DataParsing",
    "priority": "Normal",
    "parameters": {
      "type": "DataParsing",
      "parse_mode": "SpecificDbNums",
      "db_nums": "7999,8001",
      "refno": ""
    }
  }
]
```

### 应用配置

位置: `DbOption.toml`

格式:
```toml
db_path = "deployment_sites.sqlite"
surreal_host = "localhost"
surreal_port = 8000
web_host = "0.0.0.0"
web_port = 3000
log_level = "info"
max_connections = 100
```

## 常用命令

```bash
# 检查代码
cargo check --bin egui_remote_sync --features gui

# 构建发布版本
cargo build --release --bin egui_remote_sync --features gui

# 运行测试
cargo test --features gui

# 格式化代码
cargo fmt

# 代码检查
cargo clippy --features gui
```

## 调试技巧

### 启用日志

```bash
RUST_LOG=debug cargo run --bin egui_remote_sync --features gui
```

### 查看任务模板

```bash
cat ~/.config/egui_remote_sync/task_templates.json | jq
```

### 验证配置文件

```bash
cat DbOption.toml
```

## 下一步

- 查看完整文档: [EGUI_CONFIG_WIZARD_GUIDE.md](./EGUI_CONFIG_WIZARD_GUIDE.md)
- 查看实现总结: [IMPLEMENTATION_SUMMARY.md](../../.kiro/specs/egui-config-wizard/IMPLEMENTATION_SUMMARY.md)
- 查看设计文档: [design.md](../../.kiro/specs/egui-config-wizard/design.md)
- 查看需求文档: [requirements.md](../../.kiro/specs/egui-config-wizard/requirements.md)
