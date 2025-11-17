# 任务向导增强实现计划

## 概述

参考 frontend 的 deploy-wizard 实现，为 EGUI 应用创建统一的任务创建向导系统。

## 实现策略

### 阶段 1: 核心框架 (1 天)

**目标**: 建立向导框架和基础组件

**任务**:
1. 创建 `TaskWizard` trait 定义向导接口
2. 实现 `WizardManager` 管理向导状态
3. 创建通用的步骤指示器组件
4. 实现导航按钮组件
5. 创建进度条组件

**文件**:
- `src/gui/components/wizard/mod.rs`
- `src/gui/components/wizard/manager.rs`
- `src/gui/components/wizard/step_indicator.rs`
- `src/gui/components/wizard/navigation.rs`

### 阶段 2: 数据解析任务向导 (2 天)

**目标**: 实现完整的数据解析任务创建流程

**步骤 1: 项目扫描** (0.5 天)
- 目录浏览器组件
- 异步目录扫描
- 项目信息展示

**步骤 2: 项目选择** (0.5 天)
- 多选列表组件
- 项目详情面板
- 过滤和搜索

**步骤 3: 解析配置** (0.5 天)
- 解析模式选择
- 参数输入表单
- 配置验证

**步骤 4: 预览确认** (0.5 天)
- 配置摘要显示
- 资源预估
- 任务创建

**文件**:
- `src/gui/components/wizard/parse_task_wizard.rs`
- `src/gui/components/wizard/steps/project_scan.rs`
- `src/gui/components/wizard/steps/project_selection.rs`
- `src/gui/components/wizard/steps/parse_config.rs`
- `src/gui/components/wizard/steps/preview.rs`

### 阶段 3: 模型生成任务向导 (1.5 天)

**目标**: 实现模型生成任务创建流程

**步骤 1: 数据源选择** (0.5 天)
- 已解析项目列表
- 数据完整性检查

**步骤 2: 生成选项** (0.5 天)
- 生成选项配置
- 质量预设

**步骤 3: 性能配置** (0.25 天)
- 性能参数设置
- 资源预估

**步骤 4: 预览确认** (0.25 天)
- 配置摘要
- 任务创建

**文件**:
- `src/gui/components/wizard/model_gen_wizard.rs`
- `src/gui/components/wizard/steps/data_source.rs`
- `src/gui/components/wizard/steps/gen_options.rs`
- `src/gui/components/wizard/steps/performance.rs`

### 阶段 4: 集成和优化 (1 天)

**目标**: 集成到现有系统并优化用户体验

**任务**:
1. 集成到任务创建页面
2. API 客户端集成
3. 错误处理优化
4. 性能优化
5. 用户体验优化

## 关键设计决策

### 1. 状态管理

使用 Rust struct 管理向导状态，参考 React 的 useState 模式：

```rust
pub struct WizardState<T> {
    current_step: usize,
    data: T,
    validation_errors: HashMap<String, String>,
}
```

### 2. 步骤定义

使用 trait 定义步骤接口：

```rust
pub trait WizardStep {
    fn render(&mut self, ui: &mut egui::Ui, data: &mut WizardData);
    fn validate(&self, data: &WizardData) -> Result<(), HashMap<String, String>>;
    fn can_proceed(&self, data: &WizardData) -> bool;
}
```

### 3. 异步处理

使用 tokio 处理异步操作（如目录扫描）：

```rust
// 在后台任务中执行扫描
let (tx, rx) = tokio::sync::mpsc::channel(100);
tokio::spawn(async move {
    // 扫描逻辑
    tx.send(scan_result).await.ok();
});

// 在 UI 中接收结果
if let Ok(result) = rx.try_recv() {
    // 更新 UI
}
```

### 4. 组件复用

创建可复用的 UI 组件：
- `DirectoryBrowser` - 目录浏览器
- `ProjectList` - 项目列表
- `ConfigForm` - 配置表单
- `SummaryPanel` - 摘要面板

## 参考实现

### Frontend Deploy Wizard 关键代码

```typescript
// 步骤定义
const STEPS = [
  { id: 1, title: "基本信息", description: "配置环境基本信息" },
  { id: 2, title: "站点配置", description: "添加远程站点" },
  { id: 3, title: "连接测试", description: "测试连接状态" },
  { id: 4, title: "激活确认", description: "确认并激活环境" },
]

// 状态管理
const [currentStep, setCurrentStep] = useState(1)
const [wizardData, setWizardData] = useState<WizardData>({...})

// 步骤导航
const handleNext = () => {
  if (currentStep < STEPS.length) {
    setCurrentStep(currentStep + 1)
  }
}
```

### EGUI 适配

```rust
// 步骤定义
const STEPS: &[(&str, &str)] = &[
    ("基本信息", "配置环境基本信息"),
    ("站点配置", "添加远程站点"),
    ("连接测试", "测试连接状态"),
    ("激活确认", "确认并激活环境"),
];

// 状态管理
pub struct WizardState {
    current_step: usize,
    wizard_data: WizardData,
}

// 步骤导航
fn handle_next(&mut self) {
    if self.current_step < STEPS.len() - 1 {
        self.current_step += 1;
    }
}
```

## 测试计划

### 单元测试
- 状态管理逻辑
- 验证逻辑
- 数据转换逻辑

### 集成测试
- 完整向导流程
- API 调用
- 错误处理

### 用户测试
- 易用性测试
- 性能测试
- 边界情况测试

## 文档计划

1. **用户指南**: 如何使用向导创建任务
2. **开发文档**: 如何扩展向导系统
3. **API 文档**: 向导相关 API 说明

## 风险和缓解

### 风险 1: EGUI 异步处理复杂
**缓解**: 使用 channel 在异步任务和 UI 之间通信

### 风险 2: 状态管理复杂
**缓解**: 使用清晰的数据结构和状态机模式

### 风险 3: 性能问题
**缓解**: 异步扫描、分页加载、虚拟滚动

## 下一步

1. 创建详细的设计文档
2. 实现核心框架
3. 实现数据解析任务向导
4. 实现模型生成任务向导
5. 测试和优化
