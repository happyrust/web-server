# SESNO 持久化问题分析报告

## 问题现象

每次启动系统时，数据库中的 sesno 会回退到之前的值，即使增量更新已经成功执行并更新了数据。

```
[src\data_interface\increment_manager.rs:819:17] (db_no, file_latest_sesno) = (1112, 1194)
[src\data_interface\increment_manager.rs:829:17] (db_no, db_latest_sesno) = (1112, 1153)
发现需要增量更新的文件: "ams1112_0001", 当前数据库属性最大sesno: 1153, 文件属性对应sesno: 1194
```

## 根本原因分析

### 问题1: `dbnum_info_table` 与 `db_file_info` 的数据不一致

系统中存在**两个独立的 sesno 存储位置**：

1. **`dbnum_info_table`** - 按 `ref_0` 分组存储的元数据表
   - 查询方式：`SELECT VALUE sesno FROM dbnum_info_table WHERE dbnum = {dbnum}`
   - 更新方式：通过 `pe` 表的 EVENT 自动维护（`define_dbnum_event_array_id`）
   - 位置：`src/versioned_db/database.rs:477-508`

2. **`db_file_info`** - 按文件名存储的会话号书签
   - 查询方式：`SELECT sesno FROM db_file_info:{file_name}`
   - 更新方式：增量更新后手动更新（`execute_incr_update`）
   - 位置：`src/data_interface/increment_manager.rs:664-703`

### 问题2: `query_latest_sesno_by_dbnum` 只查询 `dbnum_info_table`

```rust
// src/data_interface/increment_manager.rs:752-767
async fn query_latest_sesno_by_dbnum(dbnum: u32) -> anyhow::Result<u32> {
    let mut response = SUL_DB
        .query(format!(
            r#"
            math::max(array::flatten([
                SELECT VALUE sesno FROM dbnum_info_table WHERE dbnum = {}
            ]));
            "#,
            dbnum
        ))
        .await?;
    let sesno: Option<u32> = response.take(0)?;
    Ok(sesno.unwrap_or_default())
}
```

**关键问题**：这个查询**只读取 `dbnum_info_table`**，而不读取 `db_file_info`！

### 问题3: 增量更新时 `dbnum_info_table` 未正确更新

在 `execute_incr_update` 中：

```rust
// src/data_interface/increment_manager.rs:658-660
let range_update_eles = io.collect_increment_eles(Some(sesno_range))?;
io.update_elements_to_database(&range_update_eles, true).await?;
```

`update_elements_to_database` 方法位于 `pdms_io` 包中（外部依赖），它：
- ✅ 更新了 `pe` 表中的元素数据
- ❓ **可能**触发了 EVENT 更新 `dbnum_info_table`
- ✅ 更新了 `db_file_info` 表（在 `execute_incr_update` 中手动更新）

### 问题4: EVENT 不处理 UPDATE 事件的 sesno 更新 ⚠️ **根本原因**

查看 `src/versioned_db/database.rs:477-508` 的 EVENT 定义：

```rust
DEFINE EVENT OVERWRITE update_dbnum_event ON pe WHEN $event = "CREATE" OR $event = "UPDATE" OR $event = "DELETE" THEN {
    let $is_delete = $value.deleted and $event = "UPDATE";
    let $max_sesno = if $after.sesno > $before.sesno?:0 { $after.sesno } else { $before.sesno };

    IF $event = "CREATE"   {
        UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
            dbnum: $dbnum,
            count: count?:0 + 1,
            sesno: $max_sesno,  // ✅ 更新 sesno
            max_ref1: $ref_1
        };
    } ELSE IF $event = "DELETE" OR $is_delete  {
        UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
            count: count - 1,
            sesno: $max_sesno,  // ✅ 更新 sesno
            max_ref1: $ref_1
        }
        WHERE count > 0;
    };
    // ❌ 缺少 ELSE 分支处理普通 UPDATE！
};
```

**关键发现**：
- EVENT 触发条件包含 `$event = "UPDATE"`
- 但只处理了 `$is_delete = true` 的情况（逻辑删除）
- **普通的 UPDATE 事件（属性修改）没有更新 `dbnum_info_table` 的 sesno**！
- 增量更新时，大部分操作是 `EleOperation::Modified`，会触发 UPDATE 事件
- 这些 UPDATE 事件**不会更新 `dbnum_info_table`**，导致 sesno 停留在旧值

## 数据流分析

### 全量解析流程

```
sync_pdms_with_callback()
  ├─ 1. REMOVE EVENT update_dbnum_event ON pe
  ├─ 2. 解析所有元素到 pe 表
  ├─ 3. 手动生成 UPSERT 语句更新 dbnum_info_table
  │     └─ src/versioned_db/pe.rs:281-290
  └─ 4. ❌ 未重新创建 EVENT
```

### 增量更新流程

```
execute_incr_update()
  ├─ 1. collect_increment_eles() - 收集增量元素
  │     └─ 返回: Vec<(RefNo, EleOperation::Modified, ElementData)>
  ├─ 2. update_elements_to_database() - 更新 pe 表
  │     ├─ ✅ pe 表 UPDATE 成功 (sesno=1194)
  │     ├─ 🔔 触发 EVENT: $event = "UPDATE", $is_delete = false
  │     └─ ❌ EVENT 没有 ELSE 分支，dbnum_info_table 未更新 (sesno=1153)
  ├─ 3. 手动更新 db_file_info 表
  │     └─ ✅ db_file_info:ams1112_0001 更新成功 (sesno=1194)
  └─ 4. ❌ dbnum_info_table 仍然是旧值 (sesno=1153)
```

### 启动检测流程

```
init_watcher()
  ├─ 1. 读取文件最新 sesno: 1194
  ├─ 2. 查询数据库最新 sesno
  │     └─ query_latest_sesno_by_dbnum(1112)
  │         └─ SELECT FROM dbnum_info_table WHERE dbnum = 1112
  │             └─ 返回: 1153 (旧值！)
  └─ 3. 检测到增量: 1153 < 1194
```

## 解决方案

### 方案1: 修复 EVENT 逻辑以处理 UPDATE 事件（推荐✅）

修改 `src/versioned_db/database.rs:477-508` 的 EVENT 定义，添加 ELSE 分支：

```rust
pub async fn define_dbnum_event_array_id() -> anyhow::Result<()> {
    let event_sql = r#"
DEFINE EVENT OVERWRITE update_dbnum_event ON pe WHEN $event = "CREATE" OR $event = "UPDATE" OR $event = "DELETE" THEN {
    LET $dbnum = $value.dbnum;
    LET $id = record::id($value.id);
    let $ref_0 = array::at($id, 0);
    let $ref_1 = array::at($id, 1);
    let $is_delete = $value.deleted and $event = "UPDATE";
    let $max_sesno = if $after.sesno > $before.sesno?:0 { $after.sesno } else { $before.sesno };

    IF $event = "CREATE" {
        UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
            dbnum: $dbnum,
            count: count?:0 + 1,
            sesno: $max_sesno,
            max_ref1: $ref_1
        };
    } ELSE IF $event = "DELETE" OR $is_delete {
        UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
            count: count - 1,
            sesno: $max_sesno,
            max_ref1: $ref_1
        }
        WHERE count > 0;
    } ELSE IF $event = "UPDATE" {
        -- ✅ 新增：处理普通的 UPDATE 事件
        UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
            sesno: math::max([sesno?:0, $max_sesno]),
            max_ref1: math::max([max_ref1?:0, $ref_1])
        };
    };
};
    "#;
    SUL_DB.query(event_sql).await?;
    Ok(())
}
```

**优点**：
- ✅ 根本解决问题，所有 UPDATE 都会自动更新 sesno
- ✅ 无需修改增量更新逻辑
- ✅ 符合 SurrealDB EVENT 的设计理念

**注意**：修改后需要重新运行全量解析或手动执行 `define_dbnum_event_array_id()`

### 方案2: 增量更新时手动更新 `dbnum_info_table`（临时方案）

在 `execute_incr_update` 中添加（`src/data_interface/increment_manager.rs:660` 之后）：

```rust
// 手动更新 dbnum_info_table 的 sesno
// 注意：dbnum_info_table 按 ref_0 分组，需要更新所有受影响的记录
let dbnum = basic_info.pdms_header.db_num;
let update_sql = format!(
    r#"
    UPDATE dbnum_info_table
    SET sesno = math::max([sesno?:0, {}])
    WHERE dbnum = {};
    "#,
    end_sesno, dbnum
);
SUL_DB.query(&update_sql).await
    .with_context(|| format!("更新 dbnum_info_table (dbnum={}) SESNO 失败", dbnum))?;
eprintln!("✅ 手动更新 dbnum_info_table (dbnum={}) SESNO={}", dbnum, end_sesno);
```

**优点**：
- ✅ 快速修复，无需修改 EVENT
- ✅ 可以立即部署

**缺点**：
- ❌ 治标不治本，每次增量更新都需要手动更新
- ❌ 增加了额外的数据库查询开销

### 方案3: 统一 sesno 查询逻辑（不推荐）

修改 `query_latest_sesno_by_dbnum` 同时查询两个表：

```rust
async fn query_latest_sesno_by_dbnum(dbnum: u32) -> anyhow::Result<u32> {
    let sql = format!(
        r#"
        math::max([
            math::max(array::flatten([SELECT VALUE sesno FROM dbnum_info_table WHERE dbnum = {}])),
            math::max(array::flatten([SELECT VALUE sesno FROM db_file_info WHERE dbnum = {}]))
        ]);
        "#,
        dbnum, dbnum
    );
    let mut response = SUL_DB.query(&sql).await?;
    let sesno: Option<u32> = response.take(0)?;
    Ok(sesno.unwrap_or_default())
}
```

**优点**：
- ✅ 兼容两种数据源

**缺点**：
- ❌ 没有解决根本问题，数据不一致仍然存在
- ❌ 增加查询复杂度
- ❌ `db_file_info` 表没有 `dbnum` 字段，需要修改表结构

## 推荐实施步骤

1. **立即修复**：在 `execute_incr_update` 中手动更新 `dbnum_info_table`（方案2）
2. **根本修复**：确保全量解析后重新创建 EVENT（方案1）
3. **验证修复**：添加日志验证两个表的 sesno 一致性
4. **长期优化**：考虑统一 sesno 存储，避免数据冗余

## 验证方法

```sql
-- 检查 EVENT 是否存在
SHOW EVENTS ON pe;

-- 对比两个表的 sesno
SELECT dbnum, sesno FROM dbnum_info_table WHERE dbnum = 1112;
SELECT sesno FROM db_file_info:ams1112_0001;

-- 检查 pe 表的最大 sesno
SELECT math::max(array::flatten([SELECT VALUE sesno FROM pe WHERE dbnum = 1112]));
```

