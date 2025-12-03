-- 测试远程站点配置SQL
-- 这个脚本用于在 deployment_sites.sqlite 中注册测试站点

-- 1. 创建测试环境 (Test Environment)
INSERT OR REPLACE INTO remote_sync_envs (
    id,
    name,
    mqtt_host,
    mqtt_port,
    file_server_host,
    location,
    location_dbs,
    reconnect_initial_ms,
    reconnect_max_ms,
    created_at,
    updated_at
) VALUES (
    'test-env-001',
    'Test Environment - Beijing',
    'localhost',                    -- MQTT 服务器地址
    1883,                           -- MQTT 端口
    'http://localhost:8080',        -- 文件服务器地址 (主站点)
    'beijing',                      -- Location 标识
    '1112',                         -- 监听的数据库编号
    5000,                           -- 重连初始间隔 (5秒)
    60000,                          -- 重连最大间隔 (60秒)
    datetime('now'),
    datetime('now')
);

-- 2. 创建测试站点 (Test Site - 模拟上海节点)
INSERT OR REPLACE INTO remote_sync_sites (
    id,
    env_id,
    name,
    location,
    http_host,
    dbnums,
    notes,
    created_at,
    updated_at
) VALUES (
    'test-site-shanghai',
    'test-env-001',
    'Shanghai Test Site',
    'shanghai',                                             -- 不同的 location
    'http://localhost:9090',                                -- 测试站点的HTTP地址 (不同端口)
    '1112',                                                 -- 订阅的数据库编号
    'Test site for simulating remote synchronization',
    datetime('now'),
    datetime('now')
);

-- 3. 创建另一个测试站点 (Test Site - 模拟深圳节点)
INSERT OR REPLACE INTO remote_sync_sites (
    id,
    env_id,
    name,
    location,
    http_host,
    dbnums,
    notes,
    created_at,
    updated_at
) VALUES (
    'test-site-shenzhen',
    'test-env-001',
    'Shenzhen Test Site',
    'shenzhen',                                             -- 又一个不同的 location
    'http://localhost:9091',                                -- 另一个测试站点
    '1112',
    'Another test site for multi-site synchronization',
    datetime('now'),
    datetime('now')
);

-- 验证插入
SELECT '=== Test Environment ===' as section;
SELECT * FROM remote_sync_envs WHERE id = 'test-env-001';

SELECT '=== Test Sites ===' as section;
SELECT * FROM remote_sync_sites WHERE env_id = 'test-env-001';
