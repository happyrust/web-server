## Remote Sync Reliability TODO

### 1. 新文件必需全量导入
- **位置**: `src/data_interface/increment_manager.rs` 中 `if let Some(mut old) = self.watcher.headers.get_mut(path) { … } else { … }` 的 `else` 分支。
- **改法**:
  - 将  
    ```rust
    if current_sesno > 0 {
        params.insert(
            path.clone(),
            (new_header.clone(), 1..=current_sesno),
        );
    }
    ```
    放在 `location_dbs` 判断之后，确保全量范围在通过过滤后才加入 `params`。
  - 把生成 `.cba` 的 `#[cfg(feature="mqtt")] let file_hash = { … }` 改为：
    ```rust
    #[cfg(any(feature = "mqtt", feature = "web_server"))]
    let (output, archive_hash) = { … };
    ```
    并在 `#[cfg(feature="web_server")]` 和 `#[cfg(feature="mqtt")]` 内分别使用 `archive_hash`，避免 Web-only 构建直接裁掉压缩逻辑。

### 2. Debounce 不要丢事件
- **位置**: `async_watch` 内 `let mut should_skip_event = true; …`.
- **改法**:
  - 改成记录最后一次时间并延迟处理，而不是简单 `continue`。
  - 示例：
    ```rust
    let mut should_delay = false;
    for path in &event.paths {
        if let Some(last) = last_event_times.get(path) {
            if now.duration_since(*last) < debounce_window {
                should_delay = true;
                break;
            }
        }
    }
    if should_delay {
        for path in &event.paths {
            last_event_times.insert(path.clone(), now);
        }
        continue;
    }
    ```

### 3. `GeneratedSyncArtifact` 元数据要落地
- **位置**: `try_enqueue_sync_tasks` 的 notes 构造。
- **改法**:
  - 将乱码字符串替换为标准 UTF-8。
  - 加入 `session_range`、`old_sesno/new_sesno`，例如：
    ```rust
    if artifact.is_full_sync {
        notes_parts.push("全量同步".to_string());
    } else {
        notes_parts.push("增量同步".to_string());
    }
    if let Some(db_num) = artifact.db_num {
        notes_parts.push(format!("DB#{}", db_num));
    }
    if let Some(range) = &artifact.session_range {
        notes_parts.push(format!("会话范围: {}", range));
    }
    ```

### 4. MQTT/JSON 兼容性
- `persist_pending_sync_artifacts` 写 JSON 前需确保 `assets/` 目录存在；读取旧 JSON 时要能兼容缺少新字段的情况（`serde` 的 `Option` 已可兼容，但要注意 `is_full_sync` 默认值）。

### 5. CHANGELOG/文档
- 在 `CHANGELOG.md` 追加条目，说明：添加 pending 队列、MQTT 重试、去抖及元数据。
  - 样例：
    ```
    ## 2025-11-18
    - Add pending queue for generated sync tasks (assets/pending_sync_tasks.json)
    - Add MQTT publish retry helper & watcher debounce
    - Enrich GeneratedSyncArtifact with session ranges and site notes
    ```

完成以上改动后，再运行 `cargo fmt && cargo test`，即可将方案交给 Claude 执行或自行提交。
