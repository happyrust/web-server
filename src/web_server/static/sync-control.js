// 同步控制中心前端脚本

// 全局状态
let eventSource = null;
let currentState = null;

// 初始化
document.addEventListener('DOMContentLoaded', () => {
    initializeControls();
    loadInitialState();
    // 延迟加载节点角色，确保 DOM 完全渲染
    setTimeout(() => {
        loadNodeRole();
    }, 100);
    startEventStream();
    startStatusStream(); // 使用 SSE 消息驱动替代轮询
});

// 初始化控制按钮
function initializeControls() {
    // 启动按钮（使用 DbOption.location 作为站点标识，无需用户输入）
    document.getElementById('btn-start')?.addEventListener('click', async () => {
        const response = await fetch('/api/sync/start', { method: 'POST' });
        const data = await response.json();
        if (data.location) {
            showMessage(`${data.message} (站点: ${data.location})`, data.status);
        } else {
            showMessage(data.message, data.status);
        }
    });

    // 停止按钮
    document.getElementById('btn-stop')?.addEventListener('click', async () => {
        if (!confirm('确定要停止同步服务吗？')) return;

        const response = await fetch('/api/sync/stop', { method: 'POST' });
        const data = await response.json();
        showMessage(data.message, data.status);
    });

    // 重启按钮
    document.getElementById('btn-restart')?.addEventListener('click', async () => {
        if (!confirm('确定要重启同步服务吗？')) return;

        const response = await fetch('/api/sync/restart', { method: 'POST' });
        const data = await response.json();
        showMessage(data.message, data.status);
    });

    // 暂停按钮
    document.getElementById('btn-pause')?.addEventListener('click', async () => {
        const response = await fetch('/api/sync/pause', { method: 'POST' });
        const data = await response.json();
        showMessage(data.message, data.status);
        updateControlButtons();
    });

    // 恢复按钮
    document.getElementById('btn-resume')?.addEventListener('click', async () => {
        const response = await fetch('/api/sync/resume', { method: 'POST' });
        const data = await response.json();
        showMessage(data.message, data.status);
        updateControlButtons();
    });

    // 清空队列按钮
    document.getElementById('btn-clear-queue')?.addEventListener('click', async () => {
        if (!confirm('确定要清空同步队列吗？')) return;

        const response = await fetch('/api/sync/queue/clear', { method: 'POST' });
        const data = await response.json();
        showMessage(data.message, data.status);
    });

    // 设置主节点按钮
    document.getElementById('btn-set-master')?.addEventListener('click', async () => {
        if (!confirm('确定要将当前节点设为主节点吗？主节点可以启动 MQTT Broker。')) return;

        await setNodeRole(true);
    });

    // 设置从节点按钮
    document.getElementById('btn-set-client')?.addEventListener('click', async () => {
        if (!confirm('确定要将当前节点设为从节点吗？从节点只能作为 MQTT 客户端订阅消息。')) return;

        await setNodeRole(false);
    });

    // 启动订阅按钮
    document.getElementById('btn-start-subscription')?.addEventListener('click', async () => {
        await startSubscription();
    });

    // 停止订阅按钮
    document.getElementById('btn-stop-subscription')?.addEventListener('click', async () => {
        await stopSubscription();
    });

    // 清除主节点配置按钮
    document.getElementById('btn-clear-master-config')?.addEventListener('click', async () => {
        await clearMasterConfig();
    });
}

// 加载初始状态
async function loadInitialState() {
    try {
        const response = await fetch('/api/sync/status');
        const data = await response.json();

        if (data.status === 'success') {
            currentState = data.state;
            updateUI(data.state);
        }
    } catch (error) {
        console.error('加载状态失败:', error);
    }
}

// 启动事件轮询（替代 SSE）
function startEventStream() {
    // 使用轮询替代 SSE
    setInterval(async () => {
        try {
            const response = await fetch('/api/sync/events');
            const data = await response.json();

            if (data.status === 'success' && data.events) {
                data.events.forEach(event => {
                    handleSyncEvent(event);
                });
            }
        } catch (error) {
            console.error('获取事件失败:', error);
        }
    }, 1000); // 每秒轮询一次
}

// 处理同步事件
function handleSyncEvent(event) {
    console.log('收到事件:', event);

    // 添加到日志
    addLogEntry(event);

    // 根据事件类型更新 UI
    switch (event.type) {
        case 'Started':
            updateServiceStatus(true);
            showMessage(`服务已启动: ${event.data.env_id}`, 'success');
            break;

        case 'Stopped':
            updateServiceStatus(false);
            showMessage(`服务已停止: ${event.data.reason}`, 'warning');
            break;

        case 'ConnectionChanged':
            updateConnectionStatus(event.data);
            break;

        case 'ProgressUpdate':
            updateProgress(event.data);
            break;

        case 'SyncCompleted':
            addLogEntry({
                level: 'info',
                message: `文件同步完成: ${event.data.file_path} (${event.data.duration_ms}ms)`
            });
            break;

        case 'SyncFailed':
            addLogEntry({
                level: 'error',
                message: `同步失败: ${event.data.file_path} - ${event.data.error}`
            });
            break;

        case 'Alert':
            handleAlert(event.data);
            break;

        case 'MetricsUpdate':
            updateMetrics(event.data);
            break;
    }
}

// 更新 UI
function updateUI(state) {
    // 更新服务状态
    updateServiceStatus(state.is_running, state.is_paused);

    // 更新连接状态
    updateConnectionStatus({
        mqtt_connected: state.mqtt_connected,
        watcher_active: state.watcher_active
    });

    // 更新队列长度
    document.getElementById('queue-length').textContent = state.queue_size || 0;

    // 更新性能指标
    document.getElementById('metric-sync-rate').textContent =
        (state.sync_rate_mbps || 0).toFixed(2);
    document.getElementById('metric-total-synced').textContent =
        state.total_synced || 0;

    const successRate = state.total_synced && state.total_synced + state.total_failed > 0
        ? (state.total_synced / (state.total_synced + state.total_failed) * 100).toFixed(1)
        : 0;
    document.getElementById('metric-success-rate').textContent = `${successRate}%`;

    document.getElementById('metric-uptime').textContent =
        formatUptime(state.uptime_seconds || 0);

    // 更新控制按钮状态
    updateControlButtons();
}

// 更新服务状态显示
function updateServiceStatus(isRunning, isPaused) {
    const statusEl = document.getElementById('service-status');
    const indicator = statusEl.querySelector('.status-indicator');
    const text = statusEl.querySelector('span:last-child');

    if (isRunning) {
        if (isPaused) {
            indicator.className = 'status-indicator status-warning';
            text.textContent = '已暂停';
        } else {
            indicator.className = 'status-indicator status-running';
            text.textContent = '运行中';
        }
    } else {
        indicator.className = 'status-indicator status-stopped';
        text.textContent = '已停止';
    }
}

// 更新连接状态
function updateConnectionStatus(data) {
    // MQTT 状态
    const mqttEl = document.getElementById('mqtt-status');
    const mqttIndicator = mqttEl.querySelector('.status-indicator');
    const mqttText = mqttEl.querySelector('span:last-child');

    if (data.mqtt_connected) {
        mqttIndicator.className = 'status-indicator status-running';
        mqttText.textContent = '已连接';
    } else {
        mqttIndicator.className = 'status-indicator status-stopped';
        mqttText.textContent = '未连接';
    }

    // Watcher 状态
    const watcherEl = document.getElementById('watcher-status');
    const watcherIndicator = watcherEl.querySelector('.status-indicator');
    const watcherText = watcherEl.querySelector('span:last-child');

    if (data.watcher_active) {
        watcherIndicator.className = 'status-indicator status-running';
        watcherText.textContent = '监听中';
    } else {
        watcherIndicator.className = 'status-indicator status-stopped';
        watcherText.textContent = '未激活';
    }
}

// 更新进度
function updateProgress(data) {
    document.getElementById('queue-length').textContent = data.pending || 0;
    // 可以添加进度条显示
}

// 更新性能指标
function updateMetrics(data) {
    if (data.sync_rate_mbps !== undefined) {
        document.getElementById('metric-sync-rate').textContent =
            data.sync_rate_mbps.toFixed(2);
    }
}

// 添加日志条目
function addLogEntry(event) {
    const container = document.getElementById('log-container');
    const entry = document.createElement('div');

    const timestamp = new Date().toLocaleTimeString();
    let level = 'info';
    let message = '';

    if (event.level) {
        level = event.level.toLowerCase();
        message = event.message;
    } else if (event.type) {
        message = formatEventMessage(event);
    }

    entry.className = `log-entry ${level}`;
    entry.innerHTML = `
        <span class="text-gray-500">[${timestamp}]</span>
        <span>${message}</span>
    `;

    container.insertBefore(entry, container.firstChild);

    // 保持最多 100 条日志
    while (container.children.length > 100) {
        container.removeChild(container.lastChild);
    }
}

// 格式化事件消息
function formatEventMessage(event) {
    switch (event.type) {
        case 'Started':
            return `✅ 服务启动 - 环境: ${event.data.env_id}`;
        case 'Stopped':
            return `⛔ 服务停止 - ${event.data.reason}`;
        case 'ConnectionChanged':
            return `🔄 连接状态变更 - MQTT: ${event.data.mqtt_connected ? '✅' : '❌'}, Watcher: ${event.data.watcher_active ? '✅' : '❌'}`;
        case 'SyncStarted':
            return `📤 开始同步: ${event.data.file_path}`;
        case 'SyncCompleted':
            return `✅ 同步完成: ${event.data.file_path} (${event.data.duration_ms}ms)`;
        case 'SyncFailed':
            return `❌ 同步失败: ${event.data.file_path} - ${event.data.error}`;
        default:
            return JSON.stringify(event);
    }
}

// 处理告警
function handleAlert(data) {
    const levelColors = {
        'Info': 'info',
        'Warning': 'warning',
        'Error': 'error',
        'Critical': 'error'
    };

    addLogEntry({
        level: levelColors[data.level] || 'info',
        message: `[${data.level}] ${data.message}`
    });

    // 对于严重告警，显示弹窗
    if (data.level === 'Critical' || data.level === 'Error') {
        showMessage(data.message, 'error');
    }
}

// 更新控制按钮状态
function updateControlButtons() {
    const isRunning = currentState?.is_running || false;
    const isPaused = currentState?.is_paused || false;

    document.getElementById('btn-start').disabled = isRunning;
    document.getElementById('btn-stop').disabled = !isRunning;
    document.getElementById('btn-restart').disabled = !isRunning;
    document.getElementById('btn-pause').disabled = !isRunning || isPaused;
    document.getElementById('btn-resume').disabled = !isRunning || !isPaused;
}

// 使用 SSE 实时推送状态（消息驱动）
let statusEventSource = null;

function startStatusStream() {
    // 如果已有连接，先关闭
    if (statusEventSource) {
        statusEventSource.close();
    }

    // 创建 SSE 连接
    statusEventSource = new EventSource('/api/mqtt/subscription/status/stream');

    // 监听状态更新事件
    statusEventSource.addEventListener('status', (event) => {
        try {
            const eventData = JSON.parse(event.data);
            
            // 处理 MQTT 订阅状态变化
            if (eventData.type === 'MqttSubscriptionStatusChanged') {
                const statusData = eventData.data;
                updateMqttSubscriptionStatus(statusData);
            }
            
            // 处理节点角色变化
            if (eventData.type === 'NodeRoleChanged') {
                const roleData = eventData.data;
                updateNodeRoleFromEvent(roleData);
            }
        } catch (error) {
            console.error('解析 SSE 事件失败:', error, event.data);
        }
    });

    // 连接错误处理
    statusEventSource.onerror = (error) => {
        console.error('SSE 连接错误:', error);
        // 3秒后重连
        setTimeout(() => {
            if (statusEventSource && statusEventSource.readyState === EventSource.CLOSED) {
                console.log('尝试重新连接 SSE...');
                startStatusStream();
            }
        }, 3000);
    };

    // 初始加载一次状态（确保页面加载时显示最新状态）
    loadInitialState();
}

// 更新 MQTT 订阅状态
function updateMqttSubscriptionStatus(data) {
    // 更新订阅状态显示
    const btnStart = document.getElementById('btn-start-subscription');
    const btnStop = document.getElementById('btn-stop-subscription');
    
    if (btnStart && btnStop) {
        btnStart.disabled = data.is_running;
        btnStop.disabled = !data.is_running;
    }
    
    // 更新连接状态显示
    if (data.connection_status) {
        updateConnectionStatusUI(data.connection_status);
    }
    
    // 同时更新节点角色 UI（因为状态变化可能影响角色显示）
    updateNodeRoleUI({
        location: data.location,
        is_master_node: data.is_master_node,
        node_role: data.node_role,
        master_info: data.master_info || {},
        connection_status: data.connection_status || {},
        available_masters: data.available_masters || []
    });
}

// 从事件更新节点角色
function updateNodeRoleFromEvent(data) {
    updateNodeRoleUI({
        location: data.location,
        is_master_node: data.is_master_node,
        node_role: data.node_role,
        master_info: data.master_info || {},
        connection_status: data.connection_status || {},
        available_masters: data.available_masters || []
    });
}

// 停止状态流
function stopStatusStream() {
    if (statusEventSource) {
        statusEventSource.close();
        statusEventSource = null;
    }
}

// 提示输入环境 ID
async function promptForEnvId() {
    // 先获取可用的环境列表
    try {
        const response = await fetch('/api/remote-sync/envs');
        const data = await response.json();

        if (data.items && data.items.length > 0) {
            // 如果有环境，显示选择对话框
            const envList = data.items.map(env =>
                `${env.name} (${env.id})`
            ).join('\n');

            const selectedEnv = prompt(
                `请选择要启动的环境:\n${envList}\n\n请输入环境ID:`,
                data.items[0].id
            );

            return selectedEnv;
        } else {
            alert('没有可用的环境配置，请先创建环境');
            return null;
        }
    } catch (error) {
        console.error('获取环境列表失败:', error);
        return prompt('请输入环境ID:');
    }
}

// 显示消息
function showMessage(message, type = 'info') {
    // 添加到日志
    addLogEntry({
        level: type === 'success' ? 'info' : type,
        message: message
    });

    // TODO: 可以添加更美观的通知组件
    if (type === 'error') {
        console.error(message);
    } else {
        console.log(message);
    }
}

// 格式化运行时间
function formatUptime(seconds) {
    if (seconds < 60) {
        return `${seconds}s`;
    } else if (seconds < 3600) {
        const minutes = Math.floor(seconds / 60);
        return `${minutes}m ${seconds % 60}s`;
    } else {
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        return `${hours}h ${minutes}m`;
    }
}

// ========= 节点角色管理 =========

// 加载节点角色状态
async function loadNodeRole() {
    // 检查必要的元素是否存在
    const roleBadge = document.getElementById('node-role-badge');
    if (!roleBadge) {
        // 元素不存在，可能是页面还未完全加载，稍后重试
        return;
    }

    try {
        const response = await fetch('/api/mqtt/subscription/status');
        const data = await response.json();

        if (data.status === 'success') {
            updateNodeRoleUI(data);
        }
    } catch (error) {
        console.error('加载节点角色失败:', error);
        // 只在元素存在时才显示错误消息
        const roleBadge = document.getElementById('node-role-badge');
        if (roleBadge) {
            showMessage('加载节点角色失败: ' + error.message, 'error');
        }
    }
}

// 更新节点角色 UI
function updateNodeRoleUI(data) {
    const isMaster = data.is_master_node || false;
    const location = data.location || 'unknown';
    const roleBadge = document.getElementById('node-role-badge');
    const nodeLocation = document.getElementById('node-location');
    const btnSetMaster = document.getElementById('btn-set-master');
    const btnSetClient = document.getElementById('btn-set-client');
    const roleCard = document.getElementById('node-role-card');
    const masterSelectionSection = document.getElementById('master-selection-section');
    const masterSelect = document.getElementById('master-select');
    const btnStartSubscription = document.getElementById('btn-start-subscription');
    const btnStopSubscription = document.getElementById('btn-stop-subscription');
    const subscriptionStatus = document.getElementById('subscription-status');
    const masterConnectionInfo = document.getElementById('master-connection-info');

    // 检查元素是否存在
    if (!roleBadge || !nodeLocation || !btnSetMaster || !btnSetClient || !roleCard) {
        console.warn('节点角色管理元素未找到，可能页面尚未完全加载');
        return;
    }

    // 更新角色徽章
    if (isMaster) {
        roleBadge.textContent = '主节点';
        roleBadge.className = 'px-3 py-1 rounded-full text-sm font-bold bg-purple-100 text-purple-700 border border-purple-300';
        roleCard.className = 'flex items-center gap-3 px-4 py-3 rounded-lg border border-purple-200 bg-purple-50';
        // 主节点隐藏主节点选择区域
        if (masterSelectionSection) masterSelectionSection.style.display = 'none';
    } else {
        roleBadge.textContent = '从节点';
        roleBadge.className = 'px-3 py-1 rounded-full text-sm font-bold bg-blue-100 text-blue-700 border border-blue-300';
        roleCard.className = 'flex items-center gap-3 px-4 py-3 rounded-lg border border-blue-200 bg-blue-50';
        // 从节点显示主节点选择区域
        if (masterSelectionSection) masterSelectionSection.style.display = 'block';
    }

    // 更新位置信息
    nodeLocation.textContent = `(${location})`;

    // 显示/隐藏切换按钮
    if (isMaster) {
        btnSetMaster.style.display = 'none';
        btnSetClient.style.display = 'inline-block';
    } else {
        btnSetMaster.style.display = 'inline-block';
        btnSetClient.style.display = 'none';
    }

    // 更新主节点选择列表（仅从节点）
    if (!isMaster && masterSelect && data.available_masters) {
        // 清空现有选项
        masterSelect.innerHTML = '<option value="">请选择主节点...</option>';
        
        // 添加主节点选项（只显示有有效 location 的主节点）
        data.available_masters.forEach(master => {
            const location = master.location;
            // 跳过 location 为空、undefined 或 'unknown' 的主节点
            if (!location || location === 'unknown' || location.trim() === '') {
                return;
            }
            
            const host = master.mqtt_host || '';
            const port = master.mqtt_port || 1883;
            const value = `${location}|${host}|${port}`;
            const option = document.createElement('option');
            option.value = value;
            option.textContent = `${location} (${host}:${port})`;
            
            // 如果这是当前选择的主节点，标记为选中
            if (data.selected_master && data.selected_master === `${host}:${port}`) {
                option.selected = true;
            }
            
            masterSelect.appendChild(option);
        });
    }

    // 获取清除主节点配置按钮
    const btnClearMasterConfig = document.getElementById('btn-clear-master-config');

    // 更新订阅状态
    if (!isMaster && subscriptionStatus) {
        const isRunning = data.is_subscription_running || false;
        if (isRunning) {
            subscriptionStatus.textContent = '✓ 订阅中';
            subscriptionStatus.className = 'text-sm text-green-600 font-semibold';
            if (btnStartSubscription) btnStartSubscription.style.display = 'none';
            if (btnStopSubscription) btnStopSubscription.style.display = 'inline-block';
        } else {
            subscriptionStatus.textContent = '○ 未订阅';
            subscriptionStatus.className = 'text-sm text-gray-500';
            if (btnStartSubscription) btnStartSubscription.style.display = 'inline-block';
            if (btnStopSubscription) btnStopSubscription.style.display = 'none';
        }
    }

    // 更新连接信息
    if (!isMaster && masterConnectionInfo && data.connection_status) {
        const connStatus = data.connection_status;
        if (connStatus.master_location) {
            const info = `主节点: ${connStatus.master_location} (${connStatus.master_host}:${connStatus.master_port}) - ` +
                        (connStatus.master_online ? '✓ 在线' : '✗ 离线');
            masterConnectionInfo.textContent = info;
            masterConnectionInfo.className = connStatus.master_online 
                ? 'mt-2 text-xs text-green-600' 
                : 'mt-2 text-xs text-red-600';
            
            if (connStatus.diagnostic_message) {
                masterConnectionInfo.textContent += ` - ${connStatus.diagnostic_message}`;
            }
            
            // 有主节点配置时，显示"取消订阅"按钮
            if (btnClearMasterConfig) {
                btnClearMasterConfig.style.display = 'inline-block';
            }
        } else if (connStatus.diagnostic_message) {
            masterConnectionInfo.textContent = connStatus.diagnostic_message;
            masterConnectionInfo.className = 'mt-2 text-xs text-yellow-600';
            // 没有主节点配置，隐藏"取消订阅"按钮
            if (btnClearMasterConfig) {
                btnClearMasterConfig.style.display = 'none';
            }
        } else {
            masterConnectionInfo.textContent = '';
            // 没有主节点配置，隐藏"取消订阅"按钮
            if (btnClearMasterConfig) {
                btnClearMasterConfig.style.display = 'none';
            }
        }
    } else if (btnClearMasterConfig) {
        // 主节点不需要显示此按钮
        btnClearMasterConfig.style.display = 'none';
    }
}

// 设置节点角色
async function setNodeRole(isMaster) {
    const roleLoading = document.getElementById('role-loading');
    const btnSetMaster = document.getElementById('btn-set-master');
    const btnSetClient = document.getElementById('btn-set-client');

    // 检查元素是否存在
    if (!roleLoading || !btnSetMaster || !btnSetClient) {
        console.error('节点角色管理元素未找到');
        showMessage('❌ 页面元素未加载完成，请刷新页面重试', 'error');
        return;
    }

    // 显示加载状态
    roleLoading.style.display = 'inline-block';
    btnSetMaster.disabled = true;
    btnSetClient.disabled = true;

    try {
        const endpoint = isMaster ? '/api/mqtt/node/set-master' : '/api/mqtt/node/set-client';
        const response = await fetch(endpoint, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' }
        });

        const data = await response.json();

        if (data.status === 'success') {
            showMessage('✅ ' + data.message, 'success');
            // 状态会通过 SSE 自动更新，不需要手动加载
        } else {
            showMessage('❌ ' + data.message, 'error');
        }
    } catch (error) {
        console.error('设置节点角色失败:', error);
        showMessage('❌ 设置失败: ' + error.message, 'error');
    } finally {
        // 隐藏加载状态
        roleLoading.style.display = 'none';
        btnSetMaster.disabled = false;
        btnSetClient.disabled = false;
    }
}

// 启动订阅
async function startSubscription() {
    const masterSelect = document.getElementById('master-select');
    const btnStartSubscription = document.getElementById('btn-start-subscription');
    
    if (!btnStartSubscription) {
        showMessage('❌ 页面元素未加载完成，请刷新页面重试', 'error');
        return;
    }

    // 构建请求体
    let requestBody = {};
    
    // 如果是从节点，需要选择主节点
    if (masterSelect && masterSelect.style.display !== 'none') {
        const selectedValue = masterSelect.value;
        if (!selectedValue) {
            showMessage('❌ 请先选择要订阅的主节点', 'error');
            return;
        }

        // 解析选择的值：location|host|port
        const [location, host, port] = selectedValue.split('|');
        requestBody = {
            master_location: location,
            master_mqtt_host: host,
            master_mqtt_port: parseInt(port)
        };
    }
    // 主节点启动订阅时不需要参数，发送空对象即可
    
    btnStartSubscription.disabled = true;
    btnStartSubscription.textContent = '启动中...';

    try {
        const response = await fetch('/api/mqtt/subscription/start', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(requestBody)
        });

        // 检查响应状态
        if (!response.ok) {
            // 尝试读取错误文本
            const errorText = await response.text();
            let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
            try {
                // 如果错误文本是 JSON，尝试解析
                const errorJson = JSON.parse(errorText);
                if (errorJson.message) {
                    errorMessage = errorJson.message;
                }
            } catch {
                // 如果不是 JSON，直接使用文本
                if (errorText) {
                    errorMessage = errorText.substring(0, 200); // 限制长度
                }
            }
            showMessage('❌ 启动失败: ' + errorMessage, 'error');
            return;
        }

        const data = await response.json();

        if (data.status === 'success') {
            showMessage('✅ ' + data.message, 'success');
            // 状态会通过 SSE 自动更新，不需要手动加载
        } else {
            showMessage('❌ ' + data.message, 'error');
        }
    } catch (error) {
        console.error('启动订阅失败:', error);
        showMessage('❌ 启动失败: ' + error.message, 'error');
    } finally {
        btnStartSubscription.disabled = false;
        btnStartSubscription.textContent = '▶ 启动订阅';
    }
}

// 停止订阅
async function stopSubscription() {
    const btnStopSubscription = document.getElementById('btn-stop-subscription');
    
    if (!btnStopSubscription) {
        showMessage('❌ 页面元素未加载完成，请刷新页面重试', 'error');
        return;
    }

    if (!confirm('确定要停止 MQTT 订阅吗？')) return;

    btnStopSubscription.disabled = true;
    btnStopSubscription.textContent = '停止中...';

    try {
        const response = await fetch('/api/mqtt/subscription/stop', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' }
        });

        // 检查响应状态
        if (!response.ok) {
            // 尝试读取错误文本
            const errorText = await response.text();
            let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
            try {
                // 如果错误文本是 JSON，尝试解析
                const errorJson = JSON.parse(errorText);
                if (errorJson.message) {
                    errorMessage = errorJson.message;
                }
            } catch {
                // 如果不是 JSON，直接使用文本
                if (errorText) {
                    errorMessage = errorText.substring(0, 200); // 限制长度
                }
            }
            showMessage('❌ 停止失败: ' + errorMessage, 'error');
            return;
        }

        const data = await response.json();

        if (data.status === 'success') {
            showMessage('✅ ' + data.message, 'success');
            // 状态会通过 SSE 自动更新，不需要手动加载
        } else {
            showMessage('❌ ' + data.message, 'error');
        }
    } catch (error) {
        console.error('停止订阅失败:', error);
        showMessage('❌ 停止失败: ' + error.message, 'error');
    } finally {
        btnStopSubscription.disabled = false;
        btnStopSubscription.textContent = '⏹ 停止订阅';
    }
}

// 清除主节点配置（取消订阅主节点）
async function clearMasterConfig() {
    const btnClearMasterConfig = document.getElementById('btn-clear-master-config');
    if (!btnClearMasterConfig) return;

    if (!confirm('确定要清除主节点配置吗？\n\n这将：\n• 停止当前订阅\n• 清除主节点连接信息\n• 允许重新选择其他主节点')) return;

    btnClearMasterConfig.disabled = true;
    btnClearMasterConfig.textContent = '处理中...';

    try {
        const response = await fetch('/api/mqtt/subscription/clear-master-config', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' }
        });

        if (!response.ok) {
            const errorText = await response.text();
            let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
            try {
                const errorJson = JSON.parse(errorText);
                if (errorJson.message) {
                    errorMessage = errorJson.message;
                }
            } catch {
                if (errorText) {
                    errorMessage = errorText.substring(0, 200);
                }
            }
            showMessage('❌ 清除失败: ' + errorMessage, 'error');
            return;
        }

        const data = await response.json();

        if (data.status === 'success') {
            showMessage('✅ ' + data.message, 'success');
            // 状态会通过 SSE 自动更新
            // 隐藏清除按钮，显示主节点选择
            btnClearMasterConfig.style.display = 'none';
            const masterSelect = document.getElementById('master-select');
            if (masterSelect) {
                masterSelect.value = '';
            }
        } else {
            showMessage('❌ ' + data.message, 'error');
        }
    } catch (error) {
        console.error('清除主节点配置失败:', error);
        showMessage('❌ 清除失败: ' + error.message, 'error');
    } finally {
        btnClearMasterConfig.disabled = false;
        btnClearMasterConfig.textContent = '🗑 取消订阅';
    }
}