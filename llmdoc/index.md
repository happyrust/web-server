# AIOS Database - LLM Documentation Index

本文档系统为LLM代理提供项目检索地图,按4个类别组织。

## 文档结构

### `/overview/` - 项目概览
高层次的项目背景和定位。

- (待添加项目整体概览)

### `/guides/` - 操作指南
面向开发者的分步操作说明。

- [失败任务队列使用指南](guides/failed-task-queue-usage.md) - 如何使用错误恢复系统
- [增量更新实时监控仪表盘使用指南](guides/dashboard-usage-guide.md) - 如何使用Dashboard进行实时监控和失败任务管理

### `/architecture/` - 系统架构
系统设计和模块交互的检索地图。

- [增量更新错误恢复架构](architecture/increment-error-recovery.md) - P0级错误恢复机制的完整实现
- [实时监控仪表盘架构](architecture/dashboard-architecture.md) - Dashboard系统的技术架构、组件设计和执行流程

### `/reference/` - 参考手册
事实性查找信息。

- [失败任务类型参考](reference/failed-task-types.md) - FailedTaskType枚举详细说明

## 外部文档链接

项目还包含以下传统文档(位于`/docs/`):
- `INCREMENT_DETECTION_FLOWCHART.md` - 增量检测完整流程图
- `INCREMENT_UPDATE_FLOW_ANALYSIS.md` - 增量更新流程分析
- `REMOTE_SYNC_DEVELOPMENT_GUIDE.md` - 远程同步开发指南

---

**文档版本**: 1.1
**最后更新**: 2025-11-21
**维护者**: AIOS开发团队
