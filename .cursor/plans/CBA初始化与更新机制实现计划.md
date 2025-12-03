<!-- c76f89e4-878d-4a92-a4c0-879b9c2869f9 332e46be-68ba-44b3-9800-0ab461e082e4 -->
# CBA初始化与更新机制实现计划

## 目标

1. 启动时检查CBA文件是否已初始化
2. 扫描所有数据库，为缺失的CBA文件生成初始版本
3. 检测到数据库变化时自动更新对应的CBA文件
4. 移除mqtt feature限制，改为运行时配置控制

## 实现步骤

### 1. 修改 `src/data_interface/increment_manager.rs`

#### 1.1 移除mqtt feature限制

- 移除第838行的 `#[cfg(feature = "mqtt")]` 条件编译
- 改为使用运行时配置检查：`get_db_option().sync_live.unwrap_or(false)`

#### 1.2 添加CBA文件检查辅助函数

在 `init_watcher` 方法之前添加辅助函数：

- `should_generate_cba(file_path: &Path, cba_path: &Path, file_sesno: i32, db_sesno: i32) -> bool`
- 检查CBA文件是否存在
- 如果不存在，返回true
- 如果存在，比较file_sesno和db_sesno，如果file_sesno > db_sesno，返回true
- 否则返回false

#### 1.3 修改 `init_watcher` 方法中的CBA生成逻辑

- 位置：第838-850行
- 修改前：无条件生成（在mqtt feature下）
- 修改后：

1. 检查 `sync_live` 配置是否启用
2. 调用 `should_generate_cba` 判断是否需要生成
3. 如果需要生成，执行压缩并记录日志
4. 如果不需要，记录跳过日志

#### 1.4 在增量更新后更新CBA

- 位置：`execute_incr_update` 方法执行成功后
- 检查是否有文件被更新
- 如果有，重新生成对应的CBA文件

### 2. 添加日志输出

- CBA文件已存在且无需更新的日志
- CBA文件生成成功的日志
- CBA文件更新成功的日志
- 跳过CBA生成的日志（当sync_live=false时）

### 3. 错误处理

- CBA生成失败时，记录错误但不中断启动流程
- 使用 `eprintln!` 输出错误信息
- 考虑加入失败重试队列（如果已有相关机制）

## 关键代码位置

### 主要修改文件

- `src/data_interface/increment_manager.rs`
- 第838-850行：移除feature限制，添加检查逻辑
- 添加辅助函数：`should_generate_cba`

### 相关配置

- 使用 `sync_live` 配置项控制CBA生成（已存在）
- 不需要修改配置文件结构

## 实现细节

### CBA文件路径

- 输出路径：`assets/archives/{file_name}.cba`
- 临时目录：`assets/temp`

### 判断逻辑

1. **文件不存在**：需要生成
2. **文件存在但file_sesno > db_sesno**：需要更新
3. **文件存在且file_sesno <= db_sesno**：跳过

### 性能考虑

- CBA生成是IO密集型操作，保持当前同步执行方式
- 大文件生成可能需要较长时间，添加进度日志

## 测试要点

1. 首次启动：验证所有数据库都生成了CBA文件
2. 再次启动：验证已存在的CBA文件不会被重复生成
3. 数据库更新后：验证CBA文件被正确更新
4. sync_live=false：验证CBA生成被跳过

### To-dos

- [ ] 移除init_watcher中CBA生成的mqtt feature限制，改为运行时配置检查
- [ ] 添加should_generate_cba辅助函数，检查CBA文件是否存在及是否需要更新
- [ ] 修改init_watcher方法，集成CBA检查逻辑，只在需要时生成/更新CBA
- [ ] 在execute_incr_update成功后，更新被修改文件的CBA
- [ ] 添加详细的日志输出，记录CBA生成、更新、跳过的状态