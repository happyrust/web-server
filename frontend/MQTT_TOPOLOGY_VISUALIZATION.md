# MQTT 拓扑可视化功能

## 功能概述

新增了 MQTT 网络拓扑的交互式可视化图表，用于直观展示异地协同环境中各节点的订阅关系、消息流动和在线状态。

## 核心特性

### 1. 交互式 SVG 图形
- **圆形布局算法**：节点自动按圆形排列，均匀分布
- **平移和缩放**：
  - 鼠标拖拽平移视图
  - 鼠标滚轮缩放（0.5x ~ 2x）
  - 缩放中心点智能定位
- **点击交互**：点击节点查看详细信息面板

### 2. 节点可视化

#### 主节点（环境节点）
- **紫色正方形** + **圆角边框**
- 在线时有脉冲动画
- 服务器图标（Font Awesome）
- 显示在拓扑中心位置

#### 从节点（站点节点）
- **在线**: 绿色圆圈 + 脉冲动画
- **离线**: 红色圆圈（无动画）
- MQTT 订阅图标（蓝色脉冲徽章）
- 显示消息接收计数徽章
- 环绕在主节点周围

#### 选中节点
- 蓝色高亮边框（4px 宽度）
- 右侧详情面板展开

### 3. 连接关系可视化

#### 订阅连接
- **蓝色箭头线**：表示节点间的 MQTT 订阅关系
- 箭头指向订阅者
- 连接基于 `subscribed_topics` 自动推断

#### 活跃消息流
- **黄色虚线箭头**：表示正在传输的消息
- **流动动画**：虚线偏移动画（0.5 秒循环）
- 动态生成：基于最近 5 分钟内的消息投递记录

### 4. 实时更新
- **5 秒自动刷新**：定时重新加载节点状态和消息流
- **动画过渡**：新数据无缝更新，保持视觉连续性
- **在线检测**：30 秒心跳超时自动标记离线

## 使用指南

### 访问方式
1. 启动 Web 界面
2. 点击左侧导航栏的 **"拓扑可视化"** 按钮
3. 查看交互式拓扑图

### 交互操作

| 操作 | 方法 | 效果 |
|------|------|------|
| 平移视图 | 鼠标左键拖拽 | 移动整个拓扑图 |
| 缩放视图 | 鼠标滚轮 | 放大/缩小（0.5x ~ 2x） |
| 查看节点详情 | 点击节点圆圈 | 右侧显示详情面板 |
| 重置视图 | 点击"重置视图"按钮 | 恢复默认缩放和位置 |
| 刷新数据 | 点击"刷新"按钮 | 立即重新加载数据 |

### 界面布局

```
┌────────────────────────────────────────────────────────┐
│  MQTT 拓扑可视化              [重置视图] [刷新]         │
├─────────────────────────────┬──────────────────────────┤
│                             │  节点详情面板             │
│                             │  ┌──────────────────┐   │
│        SVG 拓扑图            │  │ 节点名称          │   │
│                             │  │ 位置: bj          │   │
│   ● ──→ ●                   │  │ 状态: 在线        │   │
│    ╲   ╱                    │  │ 消息数: 42       │   │
│     ● ●                     │  │ 订阅主题:         │   │
│    ╱   ╲                    │  │  - Sync/E3d      │   │
│   ●     ●                   │  │ 最后心跳: 5秒前   │   │
│                             │  └──────────────────┘   │
│                             │                          │
└─────────────────────────────┴──────────────────────────┘
```

## 技术实现

### 组件结构

**文件位置**：`frontend/src/components/views/TopologyVisualization.vue`

#### 核心响应式数据
```javascript
const nodes = ref([]);                    // MQTT 节点列表
const messages = ref([]);                 // 消息投递记录
const selectedNode = ref(null);           // 选中的节点
const viewBox = ref({ x: 0, y: 0, scale: 1 }); // 视图变换
```

#### 计算属性

##### 1. 节点位置计算（圆形布局）
```javascript
const nodesWithPositions = computed(() => {
  const centerX = 400, centerY = 300, radius = 200;

  return nodes.value.map((node, index) => {
    const angle = (index * 2 * Math.PI) / nodes.value.length;
    return {
      ...node,
      x: centerX + radius * Math.cos(angle),
      y: centerY + radius * Math.sin(angle)
    };
  });
});
```

##### 2. 订阅连接推断
```javascript
const connections = computed(() => {
  const conns = [];

  nodesWithPositions.value.forEach(node => {
    if (node.subscribed_topics.includes('Sync/E3d')) {
      // 创建从其他节点到该节点的箭头
      nodesWithPositions.value.forEach(other => {
        if (other.location !== node.location) {
          conns.push({
            from: other.location,
            to: node.location,
            active: false
          });
        }
      });
    }
  });

  return conns;
});
```

##### 3. 活跃消息流检测
```javascript
const activeMessageFlows = computed(() => {
  const now = new Date();
  const fiveMinutesAgo = now - 5 * 60 * 1000;

  return messages.value
    .filter(msg => new Date(msg.sent_at) > fiveMinutesAgo)
    .flatMap(msg =>
      msg.receivers.map(r => ({
        from: msg.sender_location,
        to: r.location,
        active: true
      }))
    );
});
```

#### SVG 结构

```xml
<svg @mousedown="startPan" @mousemove="pan" @wheel="zoom">
  <!-- 定义箭头标记 -->
  <defs>
    <marker id="arrowhead">
      <path d="M 0 0 L 10 5 L 0 10 z" />
    </marker>
  </defs>

  <!-- 主绘图组（支持平移和缩放） -->
  <g :transform="`translate(${viewBox.x}, ${viewBox.y}) scale(${viewBox.scale})`">

    <!-- 1. 连接线层 -->
    <g v-for="conn in allConnections">
      <line :x1="from.x" :y1="from.y" :x2="to.x" :y2="to.y"
            :stroke="conn.active ? '#fbbf24' : '#3b82f6'"
            :stroke-dasharray="conn.active ? '5,5' : '0'"
            marker-end="url(#arrowhead)">
        <!-- 活跃消息流动画 -->
        <animate v-if="conn.active" attributeName="stroke-dashoffset"
                 from="0" to="10" dur="0.5s" repeatCount="indefinite" />
      </line>
    </g>

    <!-- 2. 节点层 -->
    <g v-for="node in nodesWithPositions" :transform="`translate(${node.x}, ${node.y})`">
      <!-- 在线节点脉冲效果 -->
      <circle v-if="node.is_online" r="45" class="animate-ping" fill="#10b981" opacity="0.5" />

      <!-- 主节点圆圈 -->
      <circle r="40"
              :fill="node.is_online ? '#10b981' : '#ef4444'"
              :stroke="selectedNode?.location === node.location ? '#3b82f6' : '#fff'"
              :stroke-width="selectedNode?.location === node.location ? 4 : 2" />

      <!-- 节点标签 -->
      <text y="55" text-anchor="middle" class="font-mono text-xs">
        {{ node.node_name }}
      </text>

      <!-- MQTT 订阅徽章 -->
      <g v-if="node.subscribed_topics.length > 0" transform="translate(25, -25)">
        <circle r="10" fill="#3b82f6" class="animate-pulse" />
        <text font-size="10" fill="#fff">M</text>
      </g>

      <!-- 消息计数徽章 -->
      <g v-if="node.messages_received > 0" transform="translate(-25, -25)">
        <circle r="12" fill="#f59e0b" />
        <text font-size="10" fill="#fff">{{ node.messages_received }}</text>
      </g>
    </g>
  </g>
</svg>
```

### 数据流程

```
用户访问"拓扑可视化"页面
  ↓
TopologyVisualization.vue 组件挂载
  ↓
并行调用 API：
  • GET /api/mqtt/nodes     → 获取节点状态
  • GET /api/mqtt/messages  → 获取消息投递记录
  ↓
计算节点位置（圆形布局）
  ↓
推断订阅连接关系
  ↓
检测活跃消息流（最近 5 分钟）
  ↓
渲染 SVG 图形：
  • 绘制连接线（蓝色=订阅，黄色=活跃）
  • 绘制节点圆圈（绿色=在线，红色=离线）
  • 添加徽章和标签
  ↓
每 5 秒自动刷新（setInterval）
  ↓
用户交互：
  • 拖拽平移
  • 滚轮缩放
  • 点击查看详情
```

## API 依赖

| 端点 | 用途 | 返回数据 |
|------|------|---------|
| `/api/mqtt/nodes` | 获取节点状态 | `{ success: true, nodes: [...], summary: {...} }` |
| `/api/mqtt/messages` | 获取消息投递记录 | `{ success: true, messages: [...], summary: {...} }` |

详细 API 文档参见 [MQTT_NODE_MONITORING.md](MQTT_NODE_MONITORING.md)。

## 视觉设计规范

### 颜色编码

| 元素 | 形状/颜色 | 含义 |
|------|---------|------|
| 紫色正方形 (#a855f7) | 主节点 | 环境节点（主节点），位于中心 |
| 绿色圆圈 (#10b981) | 从节点在线 | 站点节点正常运行，30秒内有心跳 |
| 红色圆圈 (#ef4444) | 离线节点 | 节点超过30秒无响应 |
| 绿色箭头 (#10b981) | 已订阅连接 | 从节点已订阅主节点 MQTT 主题 |
| 蓝色箭头 (#3b82f6) | 在线未订阅 | 从节点在线但未订阅 |
| 灰色虚线箭头 (#94a3b8) | 离线连接 | 从节点离线 |
| 黄色流动箭头 (#fbbf24) | 活跃消息流 | 最近5分钟内的消息传输 |
| 绿色 P/S 徽章 | MQTT 连接 | P=发布客户端，S=订阅客户端 |
| 橙色数字徽章 (#f59e0b) | 消息计数 | 显示接收消息数量 |

### 动画效果

| 动画 | CSS 类 / SVG 动画 | 时长 |
|------|-------------------|------|
| 在线节点脉冲 | `animate-ping` | 1秒（无限循环） |
| MQTT 徽章脉冲 | `animate-pulse` | 2秒（无限循环） |
| 消息流动 | `<animate stroke-dashoffset>` | 0.5秒（无限循环） |

### 布局参数

```javascript
const LAYOUT = {
  centerX: 400,        // 圆心 X 坐标
  centerY: 300,        // 圆心 Y 坐标
  radius: 200,         // 圆形半径
  nodeRadius: 40,      // 节点圆圈半径
  badgeRadius: 10,     // 徽章圆圈半径
  minScale: 0.5,       // 最小缩放
  maxScale: 2.0        // 最大缩放
};
```

## 集成与路由

### 主界面集成（App.vue）

```vue
<template>
  <!-- 左侧导航栏 -->
  <button @click="currentView = 'topology-viz'"
          :class="{ 'btn-active': currentView === 'topology-viz' }">
    <i class="fas fa-project-diagram"></i>
    拓扑可视化
  </button>

  <!-- 主内容区 -->
  <div v-else-if="currentView === 'topology-viz'">
    <TopologyVisualization />
  </div>
</template>

<script setup>
import TopologyVisualization from './components/views/TopologyVisualization.vue';

const viewTitle = computed(() => {
  const titles = {
    // ... 其他视图
    'topology-viz': 'MQTT 拓扑可视化'
  };
  return titles[currentView.value] || 'AIOS 数据库管理';
});
</script>
```

## 性能优化

### 1. 数据过滤
- 仅显示最近 5 分钟内的活跃消息流
- 避免绘制过多动画导致性能下降

### 2. 计算缓存
- 使用 Vue 的 `computed` 属性自动缓存计算结果
- 仅在依赖数据变化时重新计算

### 3. SVG 优化
- 使用 `<g>` 分组减少 DOM 节点
- 动画使用 CSS 类而非 JavaScript 定时器

### 4. 事件节流
- 鼠标移动事件通过 `isPanning` 标志位控制
- 滚轮缩放限制范围防止过度缩放

## 待完善功能

### 1. ⚠️ 多拓扑布局算法（优先级：中）
当前仅支持圆形布局，可扩展：
- **力导向图布局**（Force-Directed）：节点间自动调整距离
- **层次布局**（Hierarchical）：按主从关系分层显示
- **网格布局**（Grid）：规则矩阵排列

**实现建议**：
```javascript
const layoutAlgorithms = {
  circular: computeCircularLayout,
  force: computeForceDirectedLayout,
  hierarchical: computeHierarchicalLayout
};

const currentLayout = ref('circular');
const positions = computed(() => layoutAlgorithms[currentLayout.value](nodes.value));
```

### 2. ⚠️ 拓扑编辑功能（优先级：低）
- 拖拽移动单个节点
- 手动添加/删除连接
- 保存自定义布局到本地存储

### 3. ✨ 性能监控叠加（优先级：中）
在节点上显示额外指标：
- CPU 使用率
- 内存占用
- 网络延迟
- 消息队列长度

**数据来源**：扩展 `/api/mqtt/nodes` API 返回性能指标

### 4. ✨ 历史回放功能（优先级：低）
- 时间轴滑块控制显示历史某个时刻的拓扑状态
- 播放消息流动的历史记录
- 用于故障回溯分析

### 5. ✨ 导出功能（优先级：低）
- 导出 SVG 图像
- 导出 PNG/PDF 格式
- 生成拓扑报告文档

## 故障排查

### 节点位置重叠
**原因**：节点数量过多，半径设置不合理
**解决**：增大 `radius` 参数或切换到其他布局算法

### 动画卡顿
**原因**：活跃消息流过多，SVG 动画性能瓶颈
**解决**：
1. 减少活跃消息流的时间窗口（从 5 分钟改为 1 分钟）
2. 限制同时显示的动画数量
3. 使用 CSS `will-change` 属性优化渲染

### 缩放后节点模糊
**原因**：SVG 渲染精度问题
**解决**：增加 SVG `viewBox` 精度，或禁用浏览器缩放

### 数据不更新
**解决步骤**：
1. 检查浏览器控制台网络请求是否成功
2. 验证 `/api/mqtt/nodes` 和 `/api/mqtt/messages` 返回数据
3. 检查 5 秒定时器是否正常运行（`setInterval`）
4. 清空浏览器缓存并刷新

## 相关文档

- [MQTT_NODE_MONITORING.md](MQTT_NODE_MONITORING.md) - MQTT 节点监控详细文档
- [MQTT_MESSAGE_VIEWER.md](MQTT_MESSAGE_VIEWER.md) - MQTT 消息查看器使用文档
- [MQTT_MONITORING_SUMMARY.md](../MQTT_MONITORING_SUMMARY.md) - 总体实现总结
- [CLAUDE.md](../CLAUDE.md) - 项目总体开发指南

## 更新日志

### 2025-11-20
- 初始版本发布
- 实现圆形布局算法
- 实现平移和缩放交互
- 实现节点在线/离线状态可视化
- 实现订阅连接和活跃消息流动画
- 集成到主界面并通过编译测试

---

**创建日期**：2025-11-20
**版本**：v1.0
**状态**：已完成基础功能，待用户验收
