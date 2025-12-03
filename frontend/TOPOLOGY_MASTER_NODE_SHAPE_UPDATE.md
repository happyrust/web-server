# 主节点形状更新说明

## 更新内容

将拓扑可视化中的主节点（环境节点）从圆形改为正方形，以便更清晰地区分主从节点。

## 视觉设计

### 修改前
- **主节点**: 紫色圆圈
- **从节点**: 绿色/红色圆圈
- **问题**: 形状相同，仅靠颜色区分，不够直观

### 修改后
- **主节点**: 紫色正方形（带圆角）
- **从节点**: 绿色/红色圆圈
- **优势**: 形状和颜色双重区分，更加直观

## 技术实现

### SVG 代码结构

#### 主节点（正方形）
```vue
<template v-if="node.is_master_node">
  <!-- 脉冲效果（在线时） -->
  <rect
    v-if="node.is_online"
    x="-50" y="-50"
    width="100" height="100"
    rx="8"
    fill="#3b82f620"
    class="animate-ping"
  />

  <!-- 主节点正方形 -->
  <rect
    x="-40" y="-40"
    width="80" height="80"
    rx="6"
    :fill="node.is_online ? '#a855f7' : '#ef4444'"
    :stroke="node.location === selectedNode?.location ? '#3b82f6' : '#7c3aed'"
    :stroke-width="node.location === selectedNode?.location ? 4 : 3"
  />

  <!-- 服务器图标 -->
  <text y="5" class="text-2xl fill-white" text-anchor="middle">
    &#xf233; <!-- Font Awesome server icon -->
  </text>
</template>
```

#### 从节点（圆形）
```vue
<template v-else>
  <!-- 脉冲效果（在线时） -->
  <circle
    v-if="node.is_online"
    r="45"
    fill="#3b82f620"
    class="animate-ping"
  />

  <!-- 从节点圆形 -->
  <circle
    r="40"
    :fill="node.is_online ? '#10b981' : '#ef4444'"
    :stroke="node.location === selectedNode?.location ? '#3b82f6' : '#fff'"
    :stroke-width="node.location === selectedNode?.location ? 4 : 2"
  />

  <!-- 客户端图标 -->
  <text y="5" class="text-2xl fill-white" text-anchor="middle">
    &#xf233; <!-- Font Awesome icon -->
  </text>
</template>
```

## 尺寸规格

| 元素 | 主节点（正方形） | 从节点（圆形） |
|------|----------------|---------------|
| 主体尺寸 | 80×80 px | 半径 40 px |
| 脉冲尺寸 | 100×100 px | 半径 45 px |
| 圆角半径 | 6 px | N/A |
| 边框宽度（普通） | 3 px | 2 px |
| 边框宽度（选中） | 4 px | 4 px |

## 颜色方案

### 主节点
- **在线**: 紫色 `#a855f7`
- **离线**: 红色 `#ef4444`
- **边框**: 深紫色 `#7c3aed`
- **选中边框**: 蓝色 `#3b82f6`

### 从节点
- **在线**: 绿色 `#10b981`
- **离线**: 红色 `#ef4444`
- **边框**: 白色 `#fff`
- **选中边框**: 蓝色 `#3b82f6`

## 图例更新

图例中的主节点图标也相应更新为正方形：

```vue
<div class="flex items-center gap-1.5">
  <div class="w-3 h-3 rounded bg-purple-600 animate-pulse"></div>
  <span class="font-semibold">主节点</span>
</div>
```

注意：
- 使用 `rounded` 而非 `rounded-full`（正方形带圆角）
- 添加 `font-semibold` 强调主节点的重要性

## 布局算法

主节点的布局位置保持不变：
- **单个主节点**: 位于圆心 `(centerX, centerY)`
- **多个主节点**: 小圆环绕，半径 = `radius * 0.3`

从节点环绕在外圈，半径 = `radius`

## 用户体验改进

### 视觉层次
1. **形状区分**: 正方形 vs 圆形
2. **颜色区分**: 紫色 vs 绿色/红色
3. **位置区分**: 中心 vs 外圈
4. **尺寸区分**: 主节点边框更粗（3px vs 2px）

### 可访问性
- 不依赖颜色单一维度区分节点类型
- 形状差异对色盲用户友好
- 清晰的视觉层次便于快速识别

## 测试验证

### 预期效果
1. ✅ 主节点显示为紫色正方形（带圆角）
2. ✅ 从节点显示为绿色/红色圆形
3. ✅ 在线节点有脉冲动画
4. ✅ 选中节点有蓝色边框高亮
5. ✅ 图例正确显示正方形图标

### 测试步骤
1. 启动 Web 服务器
2. 访问拓扑可视化页面
3. 检查主节点是否为正方形
4. 检查从节点是否为圆形
5. 点击节点验证选中效果

## 相关文件

### 修改的文件
- `frontend/src/components/views/TopologyVisualization.vue` (主要修改)
- `frontend/MQTT_TOPOLOGY_VISUALIZATION.md` (文档更新)
- `docs/TOPOLOGY_VISUALIZATION_COMPLETE_GUIDE.md` (文档更新)

### 新增文件
- `frontend/TOPOLOGY_MASTER_NODE_SHAPE_UPDATE.md` (本文档)

## 未来优化建议

1. **自定义形状**
   - 允许用户选择不同的节点形状（圆形、正方形、菱形、六边形等）
   - 保存用户偏好设置

2. **主题支持**
   - 支持深色模式
   - 自定义颜色方案

3. **图标库扩展**
   - 支持更多 Font Awesome 图标
   - 支持自定义 SVG 图标

4. **动画增强**
   - 节点切换时的过渡动画
   - 更丰富的脉冲效果

---

**更新日期**: 2025-12-01  
**更新人**: Cascade AI  
**版本**: v1.1  
**状态**: 已完成

