# 配置文件说明

## DbOption.toml

`DbOption.toml` 是项目的主配置文件,包含数据库连接、MQTT、远程同步等敏感信息。

### 首次使用

1. 复制示例配置文件:
   ```bash
   cp DbOption.example.toml DbOption.toml
   ```

2. 根据实际环境修改 `DbOption.toml` 中的配置项:
   - `mqtt_host`: MQTT服务器地址
   - `location`: 站点位置标识
   - `location_dbs`: 允许的数据库编号列表(可选)
   - `server_release_ip`: Web服务器监听地址
   - `file_server_host`: CBA文件服务器地址

### 安全注意事项

- ⚠️ **不要提交** `DbOption.toml` 到Git仓库
- ✅ `DbOption.toml` 已添加到 `.gitignore`
- ✅ 使用 `DbOption.example.toml` 作为模板参考
- 🔒 生产环境IP地址和敏感配置仅保留在本地

### 关键配置项说明

#### 同步配置
```toml
total_sync = true          # 是否启用完整同步
incr_sync = false          # 是否启用增量同步
sync_live = true           # 是否启用实时同步(CBA生成)
```

#### MQTT配置
```toml
mqtt_host = "127.0.0.1"    # MQTT服务器地址
mqtt_port = 1883           # MQTT端口
location = "local"         # 站点位置标识
```

#### 远程同步
```toml
server_release_ip = "127.0.0.1:9099"                      # Web服务器地址
file_server_host = "http://localhost:8082/assets/archives" # CBA文件服务器
```

#### 网格精度
```toml
mesh_tol_ratio = 3         # 网格容差比率(影响所有LOD级别)
```

不同LOD级别使用相同的 `mesh_tol_ratio=3`,确保一致的模型质量。

### 环境变量覆盖

部分配置可通过环境变量覆盖(待实现):
- `AIOS_MQTT_HOST`
- `AIOS_SERVER_IP`
- `AIOS_FILE_SERVER_HOST`

### 故障排查

如果遇到配置问题:

1. 确认 `DbOption.toml` 存在且格式正确
2. 检查MQTT服务器是否可达: `telnet <mqtt_host> 1883`
3. 检查CBA文件服务器是否运行
4. 查看日志输出中的配置加载信息

### 相关文档

- [CLAUDE.md](CLAUDE.md) - 项目开发指南
- [llmdoc/](llmdoc/) - LLM文档系统
- [docs/REMOTE_SYNC_DEVELOPMENT_GUIDE.md](docs/REMOTE_SYNC_DEVELOPMENT_GUIDE.md) - 远程同步开发指南
