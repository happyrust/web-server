<template>
  <div class="h-full flex flex-col bg-base-100 rounded-xl shadow-sm border border-base-200 overflow-hidden">
    <!-- Header -->
    <div class="px-6 py-4 border-b border-base-200 bg-gradient-to-r from-green-50/50 to-emerald-50/30">
      <!-- 标题行 -->
      <div class="flex items-center justify-between mb-4">
        <div>
          <h3 class="font-bold text-xl flex items-center gap-3">
            <i class="fas fa-broadcast-tower text-green-600"></i>
            MQTT 节点实时监控
          </h3>
          <p class="text-xs text-slate-500 mt-1">
            追踪各站点的 MQTT 订阅状态和消息接收情况
          </p>
        </div>
        <button
          @click="loadData"
          class="btn btn-sm btn-primary gap-2"
          :disabled="loading"
        >
          <i class="fas fa-sync" :class="{ 'fa-spin': loading }"></i>
          刷新
        </button>
      </div>

      <!-- 控制区域：分成两行，清晰分组 -->
      <div class="flex flex-col gap-3">
        <!-- 第一行：节点角色和服务状态 -->
        <div class="flex items-center gap-3 flex-wrap">
          <!-- 节点角色 -->
          <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-white border border-base-300 shadow-sm">
            <span class="text-xs text-slate-600 font-medium">节点角色:</span>
            <span class="text-sm font-bold px-2 py-0.5 rounded" :class="mqttStatus.is_master_node ? 'text-purple-600 bg-purple-50' : 'text-blue-600 bg-blue-50'">
              {{ mqttStatus.is_master_node ? '主节点' : '从节点' }}
            </span>
            <button
              v-if="!mqttStatus.is_master_node"
              @click="setAsMasterNode"
              class="btn btn-xs btn-ghost gap-1 ml-1"
              :disabled="roleLoading"
              title="设为主节点"
            >
              <i class="fas fa-crown text-yellow-500"></i>
            </button>
            <button
              v-else
              @click="setAsClientNode"
              class="btn btn-xs btn-ghost gap-1 ml-1"
              :disabled="roleLoading"
              title="设为从节点"
            >
              <i class="fas fa-users text-blue-500"></i>
            </button>
          </div>

          <!-- MQTT Server 状态（仅主节点显示） -->
          <div v-if="mqttStatus.is_master_node" class="flex items-center gap-2 px-3 py-2 rounded-lg bg-purple-50 border border-purple-200 shadow-sm">
            <div 
              class="flex items-center gap-2"
              :title="mqttStatus.is_server_running && mqttStatus.mqtt_server_port ? `MQTT Broker 监听地址: 0.0.0.0:${mqttStatus.mqtt_server_port}` : 'MQTT Broker 未启动'"
            >
              <div
                class="w-2.5 h-2.5 rounded-full"
                :class="mqttStatus.is_server_running ? 'bg-purple-600 animate-pulse' : 'bg-slate-300'"
              ></div>
              <span class="text-xs font-semibold text-purple-700">Broker:</span>
              <span class="text-xs font-bold" :class="mqttStatus.is_server_running ? 'text-purple-700' : 'text-slate-500'">
                {{ mqttStatus.is_server_running ? '运行中' : '未启动' }}
              </span>
            </div>
            <button
              v-if="!mqttStatus.is_server_running"
              @click="startMqttServer"
              class="btn btn-xs btn-primary gap-1 ml-2"
              :disabled="mqttLoading"
            >
              <i class="fas fa-server"></i>
              启动
            </button>
            <button
              v-else
              @click="stopMqttServer"
              class="btn btn-xs btn-error gap-1 ml-2"
              :disabled="mqttLoading"
            >
              <i class="fas fa-stop"></i>
              停止
            </button>
          </div>

          <!-- MQTT 订阅状态 -->
          <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-white border border-base-300 shadow-sm">
            <div class="flex items-center gap-2">
              <div
                class="w-2.5 h-2.5 rounded-full"
                :class="mqttStatus.is_subscription_running ? 'bg-success animate-pulse' : 'bg-slate-300'"
              ></div>
              <span class="text-xs font-semibold text-slate-700">订阅:</span>
              <span class="text-xs font-bold" :class="mqttStatus.is_subscription_running ? 'text-success' : 'text-slate-500'">
                {{ mqttStatus.is_subscription_running ? '已订阅' : '未订阅' }}
              </span>
            </div>
            <button
              v-if="!mqttStatus.is_subscription_running"
              @click="startMqttSubscription"
              class="btn btn-xs btn-success gap-1 ml-2"
              :disabled="mqttLoading"
            >
              <i class="fas fa-play" :class="{ 'fa-spin': mqttLoading }"></i>
              启动订阅
            </button>
            <button
              v-else
              @click="stopMqttSubscription"
              class="btn btn-xs btn-error gap-1 ml-2"
              :disabled="mqttLoading"
            >
              <i class="fas fa-stop"></i>
              停止订阅
            </button>
          </div>
        </div>

        <!-- 第二行：统计信息 -->
        <div class="flex items-center gap-3">
          <div class="stats stats-horizontal shadow-sm bg-white border border-base-200">
            <div class="stat py-2 px-4">
              <div class="stat-title text-xs text-slate-600">在线节点</div>
              <div class="stat-value text-2xl text-success font-bold">{{ summary.online }}</div>
            </div>
            <div class="stat py-2 px-4 border-l border-base-200">
              <div class="stat-title text-xs text-slate-600">离线节点</div>
              <div class="stat-value text-2xl text-error font-bold">{{ summary.offline }}</div>
            </div>
            <div class="stat py-2 px-4 border-l border-base-200">
              <div class="stat-title text-xs text-slate-600">总节点数</div>
              <div class="stat-value text-2xl text-slate-700 font-bold">{{ summary.total }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Left: Node List -->
      <div class="w-2/5 border-r border-base-200 flex flex-col">
        <div class="px-4 py-3 border-b border-base-200 bg-slate-50/50">
          <h4 class="font-bold text-sm text-slate-700 flex items-center gap-2">
            <i class="fas fa-server"></i>
            订阅节点 ({{ nodes.length }})
          </h4>
        </div>
        <div class="flex-1 overflow-y-auto p-4">
          <div v-if="loading && nodes.length === 0" class="text-center py-16">
            <span class="loading loading-spinner loading-lg text-primary"></span>
            <p class="text-sm text-slate-500 mt-3">加载中...</p>
          </div>
          <div v-else-if="nodes.length === 0" class="text-center py-16 text-slate-400">
            <i class="fas fa-server text-5xl text-slate-200 mb-4"></i>
            <p>暂无订阅节点</p>
            <p class="text-xs mt-2">节点启动后会自动显示</p>
          </div>
          <div v-else class="space-y-3">
            <div
              v-for="node in nodes"
              :key="node.location"
              @click="selectNode(node)"
              :class="[
                'card cursor-pointer transition-all duration-200 border-2',
                selectedNode?.location === node.location
                  ? 'bg-primary/10 border-primary shadow-md'
                  : 'bg-white border-slate-200 hover:border-blue-200 hover:shadow'
              ]"
            >
              <div class="card-body p-4">
                <div class="flex justify-between items-start mb-2">
                  <div class="flex items-center gap-2">
                    <div
                      class="w-3 h-3 rounded-full"
                      :class="node.is_online ? 'bg-success animate-pulse' : 'bg-error'"
                    ></div>
                    <h5 class="font-bold text-slate-800">{{ node.node_name }}</h5>
                  </div>
                  <span
                    class="badge badge-sm"
                    :class="node.is_online ? 'badge-success' : 'badge-error'"
                  >
                    {{ node.is_online ? '在线' : '离线' }}
                  </span>
                </div>
                <div class="text-xs text-slate-600 space-y-1">
                  <div class="flex items-center gap-2">
                    <i class="fas fa-map-marker-alt w-4"></i>
                    <span class="font-mono">{{ node.location }}</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <i class="fas fa-envelope w-4"></i>
                    <span>{{ node.messages_received }} 条消息</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <i class="far fa-clock w-4"></i>
                    <span>{{ formatTime(node.last_heartbeat) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Right: Message Delivery Status -->
      <div class="flex-1 flex flex-col">
        <div class="px-4 py-3 border-b border-base-200 bg-slate-50/50 flex justify-between items-center">
          <h4 class="font-bold text-sm text-slate-700 flex items-center gap-2">
            <i class="fas fa-paper-plane"></i>
            消息投递状态
          </h4>
          <div v-if="selectedNode" class="text-xs text-slate-500">
            当前查看: <span class="font-semibold text-primary">{{ selectedNode.node_name }}</span>
          </div>
        </div>
        <div class="flex-1 overflow-y-auto p-4">
          <div v-if="!selectedNode" class="flex flex-col items-center justify-center h-full text-slate-400">
            <i class="fas fa-hand-point-left text-6xl text-slate-200 mb-6 animate-pulse"></i>
            <p class="text-lg font-medium text-slate-500">请选择左侧节点查看详情</p>
          </div>
          <div v-else-if="messages.length === 0" class="text-center py-16 text-slate-400">
            <i class="fas fa-inbox text-5xl text-slate-200 mb-4"></i>
            <p>该节点暂无消息投递记录</p>
          </div>
          <div v-else class="space-y-3">
            <div
              v-for="msg in filteredMessages"
              :key="msg.message_id"
              class="card bg-white shadow-sm border border-slate-200"
            >
              <div class="card-body p-4">
                <div class="flex justify-between items-start mb-3">
                  <div>
                    <div class="flex items-center gap-2 mb-1">
                      <i class="fas fa-broadcast-tower text-blue-500"></i>
                      <span class="font-semibold text-slate-800">来自: {{ msg.sender_location }}</span>
                      <span v-if="msg.session_range" class="badge badge-sm badge-info">
                        {{ msg.session_range }}
                      </span>
                    </div>
                    <p class="text-xs text-slate-500">
                      <i class="far fa-clock mr-1"></i>
                      {{ formatTime(msg.sent_at) }}
                    </p>
                  </div>
                  <div class="text-xs">
                    <span class="badge badge-sm badge-ghost">
                      <i class="fas fa-file-archive mr-1"></i>
                      {{ msg.file_count }} 文件
                    </span>
                  </div>
                </div>

                <!-- Receivers -->
                <div class="bg-slate-50 rounded-lg p-3 border border-slate-100">
                  <div class="text-xs font-semibold text-slate-600 mb-2">接收状态:</div>
                  <div class="flex flex-wrap gap-2">
                    <div
                      v-for="receiver in msg.receivers"
                      :key="receiver.location"
                      class="flex items-center gap-2 px-3 py-1.5 rounded-full text-xs"
                      :class="getReceiverClass(receiver)"
                    >
                      <i :class="getReceiverIcon(receiver)"></i>
                      <span class="font-mono font-semibold">{{ receiver.location }}</span>
                      <span v-if="receiver.received_at" class="text-[10px] opacity-75">
                        {{ formatShortTime(receiver.received_at) }}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue';

const nodes = ref([]);
const messages = ref([]);
const selectedNode = ref(null);
const loading = ref(false);
const mqttLoading = ref(false);
const roleLoading = ref(false);

const summary = ref({
  total: 0,
  online: 0,
  offline: 0
});

const mqttStatus = ref({
  is_subscription_running: false,
  is_server_running: false,
  mqtt_server_port: null,
  location: '',
  is_master_node: false,
  node_role: 'client'
});

const filteredMessages = computed(() => {
  if (!selectedNode.value) return messages.value;

  // 过滤出与选中节点相关的消息
  return messages.value.filter(msg => {
    // 发送者是选中节点
    if (msg.sender_location === selectedNode.value.location) return true;

    // 接收者包含选中节点
    return msg.receivers.some(r => r.location === selectedNode.value.location);
  });
});

async function loadData() {
  loading.value = true;
  try {
    // 加载 MQTT 订阅状态
    const statusRes = await fetch('/api/mqtt/subscription/status');
    const statusData = await statusRes.json();

    if (statusData.status === 'success') {
      mqttStatus.value = {
        is_subscription_running: statusData.is_subscription_running || false,
        is_server_running: statusData.is_server_running || false,
        mqtt_server_port: statusData.mqtt_server_port || null,
        location: statusData.location || '',
        is_master_node: statusData.is_master_node || false,
        node_role: statusData.node_role || 'client'
      };
    }

    // 加载节点状态
    const nodesRes = await fetch('/api/mqtt/nodes');
    const nodesData = await nodesRes.json();

    if (nodesData.success) {
      nodes.value = nodesData.nodes || [];
      summary.value = nodesData.summary || { total: 0, online: 0, offline: 0 };
    }

    // 加载消息投递状态
    const messagesRes = await fetch('/api/mqtt/messages?limit=100');
    const messagesData = await messagesRes.json();

    if (messagesData.success) {
      messages.value = messagesData.messages || [];
    }
  } catch (error) {
    console.error('加载 MQTT 监控数据失败:', error);
  } finally {
    loading.value = false;
  }
}

async function startMqttSubscription() {
  mqttLoading.value = true;
  let rawResponse = '';
  try {
    const res = await fetch('/api/mqtt/subscription/start', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({})
    });
    rawResponse = await res.text();
    let data = {};
    try {
      data = rawResponse ? JSON.parse(rawResponse) : {};
    } catch (err) {
      throw new Error(rawResponse || err.message);
    }

    if (res.ok && data.status === 'success') {
      alert('✅ ' + (data.message || '订阅已启动'));
      // 重新加载数据
      await loadData();
    } else {
      const message = data.message || rawResponse || '启动失败';
      alert('❌ ' + message);
    }
  } catch (error) {
    console.error('启动 MQTT 订阅失败:', error);
    alert('❌ 启动失败: ' + error.message);
  } finally {
    mqttLoading.value = false;
  }
}

async function stopMqttSubscription() {
  if (!confirm('确定要停止 MQTT 订阅吗？停止后将不再接收远程消息。')) {
    return;
  }

  mqttLoading.value = true;
  try {
    const res = await fetch('/api/mqtt/subscription/stop', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await res.json();

    if (data.status === 'success') {
      alert('✅ ' + data.message);
      await loadData();
    } else {
      alert('❌ ' + data.message);
    }
  } catch (error) {
    console.error('停止 MQTT 订阅失败:', error);
    alert('❌ 停止失败: ' + error.message);
  } finally {
    mqttLoading.value = false;
  }
}

async function startMqttServer() {
  mqttLoading.value = true;
  try {
    const res = await fetch('/api/sync/mqtt/start', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ port: 1883 })
    });
    const data = await res.json();

    if (data.status === 'success') {
      alert('✅ ' + data.message);
      await loadData();
    } else {
      alert('❌ ' + data.message);
    }
  } catch (error) {
    console.error('启动 MQTT Broker 失败:', error);
    alert('❌ 启动失败: ' + error.message);
  } finally {
    mqttLoading.value = false;
  }
}

async function stopMqttServer() {
  if (!confirm('确定要停止 MQTT Broker 吗？')) {
    return;
  }

  mqttLoading.value = true;
  try {
    const res = await fetch('/api/sync/mqtt/stop', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await res.json();

    if (data.status === 'success') {
      alert('✅ ' + data.message);
      await loadData();
    } else {
      alert('❌ ' + data.message);
    }
  } catch (error) {
    console.error('停止 MQTT Broker 失败:', error);
    alert('❌ 停止失败: ' + error.message);
  } finally {
    mqttLoading.value = false;
  }
}

async function setAsMasterNode() {
  if (!confirm('确定要将当前节点设为主节点吗？主节点可以启动 MQTT Broker。')) {
    return;
  }

  roleLoading.value = true;
  try {
    const res = await fetch('/api/mqtt/node/set-master', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await res.json();

    if (data.status === 'success') {
      alert('✅ ' + data.message);
      await loadData();
    } else {
      alert('❌ ' + data.message);
    }
  } catch (error) {
    console.error('设置主节点失败:', error);
    alert('❌ 设置失败: ' + error.message);
  } finally {
    roleLoading.value = false;
  }
}

async function setAsClientNode() {
  if (!confirm('确定要将当前节点设为从节点吗？从节点只能作为 MQTT 客户端订阅消息。')) {
    return;
  }

  roleLoading.value = true;
  try {
    const res = await fetch('/api/mqtt/node/set-client', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await res.json();

    if (data.status === 'success') {
      alert('✅ ' + data.message);
      await loadData();
    } else {
      alert('❌ ' + data.message);
    }
  } catch (error) {
    console.error('设置从节点失败:', error);
    alert('❌ 设置失败: ' + error.message);
  } finally {
    roleLoading.value = false;
  }
}

function selectNode(node) {
  selectedNode.value = node;
}

function getReceiverClass(receiver) {
  switch (receiver.status) {
    case 'completed':
      return 'bg-success/20 text-success border border-success/30';
    case 'received':
    case 'processing':
      return 'bg-info/20 text-info border border-info/30';
    case 'failed':
      return 'bg-error/20 text-error border border-error/30';
    default:
      return 'bg-slate-100 text-slate-600 border border-slate-200';
  }
}

function getReceiverIcon(receiver) {
  switch (receiver.status) {
    case 'completed':
      return 'fas fa-check-circle';
    case 'received':
      return 'fas fa-envelope-open';
    case 'processing':
      return 'fas fa-spinner fa-spin';
    case 'failed':
      return 'fas fa-exclamation-circle';
    default:
      return 'far fa-clock';
  }
}

function formatTime(timestamp) {
  if (!timestamp) return '未知';

  try {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now - date;

    if (diff < 60 * 1000) {
      return '刚刚';
    } else if (diff < 60 * 60 * 1000) {
      return `${Math.floor(diff / 60000)} 分钟前`;
    } else if (diff < 24 * 60 * 60 * 1000) {
      return `${Math.floor(diff / 3600000)} 小时前`;
    } else {
      return date.toLocaleDateString('zh-CN');
    }
  } catch (e) {
    return timestamp;
  }
}

function formatShortTime(timestamp) {
  if (!timestamp) return '';

  try {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
  } catch (e) {
    return '';
  }
}

let refreshInterval = null;

onMounted(() => {
  loadData();
  // 每 5 秒自动刷新
  refreshInterval = setInterval(loadData, 5000);
});

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval);
  }
});
</script>

<style scoped>
.animate-pulse {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}
</style>
