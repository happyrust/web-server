# 站点卡片布局设计

## 概述

站点配置页面已从传统的表格布局改为现代化的卡片布局，提供更好的视觉效果和用户体验。

## 设计特点

### 1. 卡片式布局

每个站点显示为一个独立的卡片，包含：

```
┌─────────────────────────────────────────┐
│ 站点名称                    ⚪ 未部署   │
│ 🏷️ 环境名称                            │
├─────────────────────────────────────────┤
│ 📍 地区                                 │
│ 🌐 HTTP 地址                            │
│ 💾 DB: 数据库编号                       │
│ 🕐 创建时间                             │
├─────────────────────────────────────────┤
│ 📝 备注信息（第一行）                   │
├─────────────────────────────────────────┤
│ [✏️ 编辑] [🔌 测试]        [🗑️ 删除]  │
└─────────────────────────────────────────┘
```

### 2. 响应式网格

- **自动计算**: 根据窗口宽度自动计算每行显示的卡片数量
- **固定宽度**: 每个卡片宽度固定为 350px
- **自适应**: 窗口缩放时自动调整布局

### 3. 视觉设计

#### 颜色方案
- **背景色**: `rgb(248, 250, 252)` - 浅灰蓝色
- **边框色**: `rgb(226, 232, 240)` - 中灰色
- **文字色**: 默认黑色，次要信息使用灰色

#### 圆角和间距
- **卡片圆角**: 8px
- **内边距**: 16px
- **卡片间距**: 10px

#### 图标系统
- 📍 地区
- 🌐 HTTP 地址
- 💾 数据库
- 🕐 时间
- 🏷️ 环境标签
- 📝 备注
- ⚪ 状态指示器

### 4. 交互设计

#### 操作按钮
- **✏️ 编辑**: 打开编辑对话框
- **🔌 测试**: 测试站点连接
- **🗑️ 删除**: 删除站点（需确认）

#### 状态指示
- ⚪ 未部署（灰色）
- 🟢 运行中（绿色）
- 🟡 警告（黄色）
- 🔴 错误（红色）

## 实现细节

### 核心代码

```rust
fn render_site_card(&mut self, ui: &mut egui::Ui, site: &RemoteSyncSite, state: &AppState) {
    let card_width = 350.0;
    
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(248, 250, 252))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240)))
        .rounding(egui::Rounding::same(8))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            // 卡片内容
        });
}
```

### 响应式布局

```rust
// 计算每行卡片数量
let available_width = ui.available_width();
let card_width = 350.0;
let spacing = 10.0;
let cards_per_row = ((available_width + spacing) / (card_width + spacing))
    .floor()
    .max(1.0) as usize;
```

### 网格渲染

```rust
ui.horizontal_wrapped(|ui| {
    for (idx, site) in state.sites.iter().enumerate() {
        self.render_site_card(ui, site, state);
        
        // 添加卡片间距
        if (idx + 1) % cards_per_row != 0 && idx < state.sites.len() - 1 {
            ui.add_space(spacing);
        }
    }
});
```

## 用户体验优化

### 1. 空状态

当没有站点时，显示友好的提示：

```
        暂无站点
点击上方 ➕ 添加站点 按钮创建新站点
```

### 2. 信息层次

- **主要信息**: 站点名称（加粗）
- **次要信息**: 环境、地区、地址等
- **辅助信息**: 备注、创建时间

### 3. 操作便捷性

- 所有操作按钮在卡片底部
- 删除按钮靠右，避免误操作
- 按钮使用图标+文字，清晰易懂

## 与表格布局对比

### 表格布局（旧）

**优点**:
- 信息密度高
- 适合大量数据
- 易于排序和筛选

**缺点**:
- 视觉单调
- 移动端不友好
- 信息展示受限

### 卡片布局（新）

**优点**:
- 视觉美观
- 信息展示灵活
- 响应式友好
- 易于扫描

**缺点**:
- 信息密度较低
- 不适合超大量数据
- 排序功能需额外实现

## 未来改进

### 短期（1-2 周）

1. **状态实时更新**: 显示站点真实状态
2. **快速操作**: 添加更多快捷操作
3. **卡片悬停效果**: 鼠标悬停时高亮

### 中期（1-2 月）

1. **拖拽排序**: 支持拖拽调整顺序
2. **批量操作**: 支持多选和批量操作
3. **过滤和搜索**: 添加过滤和搜索功能

### 长期（3-6 月）

1. **自定义视图**: 支持表格/卡片切换
2. **卡片模板**: 支持自定义卡片显示内容
3. **动画效果**: 添加平滑的过渡动画

## 技术细节

### EGUI 限制

1. **布局系统**: EGUI 的布局系统与 Web 不同
2. **样式系统**: 不支持 CSS，需要手动设置
3. **响应式**: 需要手动计算和调整

### 解决方案

1. **使用 Frame**: 创建卡片容器
2. **计算布局**: 动态计算卡片数量
3. **Grid 布局**: 使用 Grid 组织卡片内容

## 示例代码

### 完整的卡片渲染

```rust
fn render_site_card(&mut self, ui: &mut egui::Ui, site: &RemoteSyncSite, state: &AppState) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(248, 250, 252))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240)))
        .rounding(egui::Rounding::same(8))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(350.0);
            
            // 标题和状态
            ui.horizontal(|ui| {
                ui.strong(&site.name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    ui.colored_label(egui::Color32::GRAY, "⚪ 未部署");
                });
            });
            
            // 详细信息
            egui::Grid::new(format!("site_card_{}", site.id))
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("📍");
                    ui.label(&site.location);
                    ui.end_row();
                    // ... 更多信息
                });
            
            // 操作按钮
            ui.horizontal(|ui| {
                if ui.button("✏️ 编辑").clicked() {
                    // 编辑逻辑
                }
                if ui.button("🔌 测试").clicked() {
                    // 测试逻辑
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑️ 删除").clicked() {
                        // 删除逻辑
                    }
                });
            });
        });
}
```

## 测试建议

### 功能测试

1. ✅ 卡片正确显示所有信息
2. ✅ 响应式布局正常工作
3. ✅ 操作按钮功能正常
4. ✅ 空状态显示正确

### 视觉测试

1. ✅ 卡片样式美观
2. ✅ 间距和对齐正确
3. ✅ 颜色搭配合理
4. ✅ 图标显示清晰

### 性能测试

1. ✅ 大量站点时性能良好
2. ✅ 窗口缩放流畅
3. ✅ 滚动性能良好

## 参考资源

- **EGUI 文档**: https://docs.rs/egui/
- **设计灵感**: Material Design Cards
- **图标系统**: Unicode Emoji

---

**最后更新**: 2025-11-17  
**维护者**: EGUI 开发团队  
**状态**: ✅ 已实现
