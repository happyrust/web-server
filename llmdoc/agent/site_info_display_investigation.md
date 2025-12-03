<!-- 站点信息显示实现方式调查报告 -->

### Code Sections (The Evidence)

#### 核心实现文件
- `src/web_server/templates/incremental_update_vue.html` (行 24-117): 站点信息角标的HTML结构、CSS样式和JavaScript加载逻辑
- `src/web_server/site_config_handlers.rs` (行 56-79): `get_site_config()` API处理器,返回DbOption配置数据
- `src/web_server/site_config_handlers.rs` (行 149-186): `read_config()` 函数,从DbOption.toml读取站点配置并映射为SiteConfig结构

#### 配置数据来源
- `site-main/config/DbOption.toml` (行 26, 28, 22): 配置文件包含 `project_name = "AvevaMarineSample"`, `project_code = '1516'`, `included_projects`等字段
- `src/web_server/site_config_handlers.rs` (行 16-54): `SiteConfig` 结构定义,包含 `location`(地区标识)、`project_name`(项目名称)、`sync_live`(主站标识)等字段

---

### Report (The Answers)

#### result

**1. 站点信息显示位置和文件**
- **文件路径**: `src/web_server/templates/incremental_update_vue.html`
- **HTML元素**: `<div id="site-meta-badge">` (行 81-90)
- **定位**: 固定定位,右上角 `position: fixed; top: 12px; right: 12px;` (行 27-29)

**2. 显示内容结构**
当前实现显示三个信息块:
- **当前站点**: `siteName + " / " + location` (如: "AvevaMarineSample / sjz") (行 105)
- **角色/地区**: chip样式标签,显示"主站(可发布 MQTT)"或"从站(订阅)" (行 101)
- **访问地址**: 当前页面的origin (如: "http://127.0.0.1:18080") (行 111)

**3. 当前显示逻辑**
- **初始状态**: `style="display:none;"` (行 81) - 默认隐藏
- **加载完成后**: 异步请求`/api/site-config`成功后设置`el.style.display = "block"` (行 112) - **一直显示**
- **无hover交互**: 当前实现中没有任何hover机制

**4. 技术栈**
- **前端框架**: 原生JavaScript (非Alpine.js),使用async/await异步加载
- **CSS框架**: 自定义样式,使用线性渐变背景 `linear-gradient(135deg, #0f172a, #1e293b)`
- **API调用**: `fetch("/api/site-config", { cache: "no-store" })`

---

#### conclusions

1. **站点信息角标默认一直显示**: 加载完成后始终可见(`display: block`),没有hover显示/隐藏逻辑
2. **数据源为DbOption.toml配置**: 通过 `/api/site-config` API读取`project_name`, `location`, `sync_live`等字段
3. **固定定位在右上角**: CSS使用`position: fixed; top: 12px; right: 12px; z-index: 9999;`
4. **角色判断依据**: `sync_live`字段为`true`时显示"主站(可发布 MQTT)",否则显示"从站(订阅)"
5. **样式采用chip设计**: 主站使用绿色chip(`master` class),从站使用蓝色chip(`replica` class)
6. **响应式布局**: 小屏幕(<640px)时角标移至底部 `bottom: 12px; left: 12px;`

---

#### relations

**数据流关系**:
```
DbOption.toml (配置文件)
    ↓ 启动时加载
aios_core::get_db_option() (全局配置)
    ↓ 读取
src/web_server/site_config_handlers.rs::read_config()
    ↓ 映射为SiteConfig结构
GET /api/site-config (HTTP API)
    ↓ 返回JSON
incremental_update_vue.html (前端异步请求)
    ↓ 解析并渲染
#site-meta-badge (DOM元素,display: block)
```

**文件关系**:
- `incremental_update_vue.html` 内联了站点信息角标的完整实现(HTML+CSS+JS),无需外部依赖
- `site_config_handlers.rs` 提供API支持,直接调用 `aios_core::get_db_option()` 获取全局配置
- `DbOption.toml` 作为唯一配置源,包含所有站点元数据

**样式逻辑关系**:
- `.label` 类: 小号灰色标签 (`color: #94a3b8`)
- `.value` 类: 粗体白色值 (`font-weight: 700; color: #e2e8f0`)
- `.chip.master` 类: 绿色渐变背景,表示主站角色
- `.chip.replica` 类: 蓝色渐变背景,表示从站角色

---

### 修改方案建议

**需求**: 默认仅显示站点名称,hover时展开显示完整信息

**方案1: 纯CSS实现 (推荐)**
```css
/* 默认状态: 仅显示站点名称 */
#site-meta-badge {
    max-height: 50px;
    overflow: hidden;
    transition: max-height 0.3s ease;
}

#site-meta-badge:hover {
    max-height: 200px; /* 展开完整高度 */
}

/* 默认隐藏角色和访问地址 */
#site-meta-badge .label:not(:first-child),
#site-meta-badge .chip,
#site-meta-badge #site-meta-url {
    opacity: 0;
    transition: opacity 0.3s ease;
}

#site-meta-badge:hover .label,
#site-meta-badge:hover .chip,
#site-meta-badge:hover #site-meta-url {
    opacity: 1;
}
```

**方案2: Alpine.js实现 (更灵活)**
如果需要更复杂的交互(如点击固定展开/收起),可引入Alpine.js:
```html
<div id="site-meta-badge" x-data="{ expanded: false }" @mouseenter="expanded = true" @mouseleave="expanded = false">
    <div class="label">当前站点</div>
    <div class="value" id="site-meta-name">加载中...</div>

    <template x-show="expanded" x-transition>
        <div class="label" style="margin-top:6px;">角色 / 地区</div>
        <div class="chip replica" id="site-meta-role">...</div>
        <div class="label" style="margin-top:6px;">访问地址</div>
        <div class="value" id="site-meta-url"></div>
    </template>
</div>
```

**实施位置**: `src/web_server/templates/incremental_update_vue.html` 第 25-90 行的 `<style>` 和 `<div>` 块

---

**文档版本**: 1.0
**创建时间**: 2025-11-22
**调查者**: scout agent
**相关文件**: `incremental_update_vue.html`, `site_config_handlers.rs`, `DbOption.toml`
