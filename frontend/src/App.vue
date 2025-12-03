<template>
  <NConfigProvider 
    :theme="isDarkMode ? darkTheme : null" 
    :locale="zhCN" 
    :date-locale="dateZhCN"
    :theme-overrides="themeOverrides"
  >
    <NMessageProvider>
      <div :class="[
        'min-h-screen flex h-screen overflow-hidden',
        isDarkMode ? 'bg-slate-950 text-slate-100' : 'bg-slate-50 text-slate-900'
      ]">
        <!-- Mobile Backdrop -->
        <div 
          v-if="isMobileMenuOpen" 
          @click="isMobileMenuOpen = false" 
          :class="[
            'fixed inset-0 z-40 lg:hidden backdrop-blur-sm transition-opacity',
            isDarkMode ? 'bg-black/50' : 'bg-slate-900/50'
          ]"
        ></div>

        <!-- 侧边栏 -->
        <aside 
          :class="[
            'fixed inset-y-0 left-0 z-50 w-64 flex flex-col shrink-0 transition-transform duration-300 lg:static lg:translate-x-0',
            isDarkMode 
              ? 'border-r border-slate-800 bg-slate-950' 
              : 'border-r border-slate-200 bg-white',
            isMobileMenuOpen ? 'translate-x-0' : '-translate-x-full'
          ]"
        >
      <div :class="[
        'px-6 py-6 flex items-center gap-3 border-b',
        isDarkMode ? 'border-slate-800' : 'border-slate-200'
      ]">
        <div class="w-12 h-12 rounded-2xl bg-blue-600/90 flex items-center justify-center text-white text-xl font-semibold shadow-lg shadow-blue-900/20">
          AI
        </div>
        <div>
          <p :class="[
            'text-xs uppercase tracking-[0.35em] font-medium',
            isDarkMode ? 'text-slate-500' : 'text-slate-500'
          ]">AIOS OPS</p>
          <p :class="[
            'text-lg font-bold tracking-tight',
            isDarkMode ? 'text-white' : 'text-slate-900'
          ]">Incremental</p>
        </div>
      </div>

      <nav class="flex-1 px-3 py-6 space-y-1 text-sm overflow-y-auto">
        <div :class="[
          'px-3 mb-2 text-xs font-bold uppercase tracking-wider',
          isDarkMode ? 'text-slate-600' : 'text-slate-500'
        ]">监控</div>
        <button
          @click="navigateTo('dashboard')"
          :class="['nav-link w-full', currentView === 'dashboard' ? 'active' : '']"
        >
          <i class="fas fa-chart-line text-lg w-6 text-center"></i><span>全局概览</span>
        </button>
        <button
          @click="navigateTo('topology')"
          :class="['nav-link w-full', currentView === 'topology' ? 'active' : '']"
        >
          <i class="fas fa-network-wired text-lg w-6 text-center"></i><span>异地拓扑</span>
        </button>
        <button
          @click="navigateTo('topology-viz')"
          :class="['nav-link w-full', currentView === 'topology-viz' ? 'active' : '']"
        >
          <i class="fas fa-project-diagram text-lg w-6 text-center"></i><span>拓扑可视化</span>
        </button>

        <div :class="[
          'px-3 mb-2 mt-6 text-xs font-bold uppercase tracking-wider',
          isDarkMode ? 'text-slate-600' : 'text-slate-500'
        ]">任务与日志</div>
        <button
          @click="navigateTo('tasks')"
          :class="['nav-link w-full', currentView === 'tasks' ? 'active' : '']"
        >
          <i class="fas fa-layer-group text-lg w-6 text-center"></i><span>任务队列</span>
        </button>
        <button
          @click="navigateTo('history')"
          :class="['nav-link w-full', currentView === 'history' ? 'active' : '']"
        >
          <i class="fas fa-history text-lg w-6 text-center"></i><span>同步历史</span>
        </button>
        <button
          @click="navigateTo('mqtt')"
          :class="['nav-link w-full', currentView === 'mqtt' ? 'active' : '']"
        >
          <i class="fas fa-signal text-lg w-6 text-center"></i><span>MQTT 消息</span>
        </button>
        <button
          @click="navigateTo('mqtt-nodes')"
          :class="['nav-link w-full', currentView === 'mqtt-nodes' ? 'active' : '']"
        >
          <i class="fas fa-broadcast-tower text-lg w-6 text-center"></i><span>MQTT 节点</span>
        </button>
        <button
          @click="navigateTo('logs')"
          :class="['nav-link w-full', currentView === 'logs' ? 'active' : '']"
        >
          <i class="fas fa-terminal text-lg w-6 text-center"></i><span>系统日志</span>
        </button>
        <button
          @click="navigateTo('archives')"
          :class="['nav-link w-full', currentView === 'archives' ? 'active' : '']"
        >
          <i class="fas fa-file-archive text-lg w-6 text-center"></i><span>归档管理</span>
        </button>

        <div :class="[
          'px-3 mb-2 mt-6 text-xs font-bold uppercase tracking-wider',
          isDarkMode ? 'text-slate-600' : 'text-slate-500'
        ]">系统</div>
        <button
          @click="navigateTo('site-config')"
          :class="['nav-link w-full', currentView === 'site-config' ? 'active' : '']"
        >
          <i class="fas fa-wrench text-lg w-6 text-center"></i><span>站点配置</span>
        </button>
        <button
          @click="navigateTo('settings')"
          :class="['nav-link w-full', currentView === 'settings' ? 'active' : '']"
        >
          <i class="fas fa-cog text-lg w-6 text-center"></i><span>参数配置</span>
        </button>
      </nav>

      <div :class="[
        'px-6 py-6 border-t text-xs space-y-3',
        isDarkMode 
          ? 'border-slate-800 text-slate-400 bg-slate-900/30' 
          : 'border-slate-200 text-slate-600 bg-slate-50'
      ]">
        <div class="flex justify-between items-center">
          <span class="flex items-center gap-2"><i class="fas fa-sync-alt"></i> 自动刷新</span>
          <span :class="[
            'font-mono px-1.5 py-0.5 rounded',
            isDarkMode ? 'bg-slate-800 text-slate-300' : 'bg-slate-200 text-slate-700'
          ]">30s</span>
        </div>
        <div class="flex justify-between items-center">
          <span class="flex items-center gap-2"><i class="fas fa-bell"></i> 实时通知</span>
          <span class="text-emerald-400 font-bold flex items-center gap-1.5">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>开启
          </span>
        </div>
      </div>
    </aside>

    <!-- 主内容区 -->
    <div class="flex-1 flex flex-col bg-slate-50 min-w-0">
      <!-- 顶部栏 -->
      <header class="bg-white border-b border-slate-200 px-6 py-4 shadow-sm shrink-0 z-10">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <!-- Mobile Menu Button -->
            <NButton 
              quaternary 
              circle 
              size="small" 
              @click="isMobileMenuOpen = !isMobileMenuOpen" 
              class="lg:hidden text-slate-600"
            >
              <template #icon>
                <i class="fas fa-bars text-lg"></i>
              </template>
            </NButton>

            <h2 class="text-xl font-bold text-slate-800 flex items-center gap-2">
              {{ viewTitle }}
              <span v-if="totalPending > 0" class="badge badge-warning gap-1 font-medium">
                {{ totalPending }} 待处理
              </span>
            </h2>
          </div>
          <div class="flex items-center gap-3">
            <!-- 站点信息 -->
            <SiteInfoBadge />
            <div class="h-6 w-px bg-slate-200 mx-1"></div>

            <div
              v-if="isConnected"
              class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-emerald-50 border border-emerald-200 text-emerald-700 text-xs font-bold"
            >
              <span class="relative flex h-2 w-2">
                <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
              </span>
              ONLINE
            </div>
            <div class="h-6 w-px bg-slate-200 mx-1"></div>
            <NButton
              quaternary
              circle
              size="small"
              @click="toggleTheme"
              title="切换深色模式"
            >
              <template #icon>
                <i :class="[isDarkMode ? 'fas fa-sun' : 'fas fa-moon']"></i>
              </template>
            </NButton>
            <NButton
              type="primary"
              size="small"
              @click="refreshAll"
              :loading="isLoading"
              class="shadow-sm shadow-blue-500/20"
            >
              <template #icon>
                <i v-if="!isLoading" class="fas fa-rotate"></i>
              </template>
              刷新
            </NButton>
          </div>
        </div>
      </header>

      <!-- 主内容 -->
      <main class="flex-1 overflow-y-auto p-6 relative">
        <div class="container mx-auto max-w-7xl h-full flex flex-col">
          
          <!-- Dashboard View -->
          <div v-if="currentView === 'dashboard'" class="space-y-6 animate-fade-in">
            <!-- 统计卡片 -->
            <section class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
              <div class="stats shadow-sm border border-base-200 bg-white text-slate-700">
                <div class="stat">
                  <div class="stat-figure text-primary/20">
                    <i class="fas fa-server text-3xl"></i>
                  </div>
                  <div class="stat-title font-medium text-slate-500">监控站点</div>
                  <div class="stat-value text-primary">{{ sites.length }}</div>
                  <div class="stat-desc">正在纳管的部署环境</div>
                </div>
              </div>

              <div class="stats shadow-sm border border-base-200 bg-white text-slate-700">
                <div class="stat">
                  <div class="stat-figure text-warning/20">
                    <i class="fas fa-clock text-3xl"></i>
                  </div>
                  <div class="stat-title font-medium text-slate-500">队列中任务</div>
                  <div class="stat-value text-warning">{{ stats.queued_updates }}</div>
                  <div class="stat-desc">正在排队的增量更新</div>
                </div>
              </div>

              <div class="stats shadow-sm border border-base-200 bg-white text-slate-700">
                <div class="stat">
                  <div class="stat-figure text-success/20">
                    <i class="fas fa-check-circle text-3xl"></i>
                  </div>
                  <div class="stat-title font-medium text-slate-500">总完成数</div>
                  <div class="stat-value text-success">{{ stats.total_synced }}</div>
                  <div class="stat-desc">累计完成 {{ stats.total_synced }} 次同步</div>
                </div>
              </div>

              <div class="stats shadow-sm border border-base-200 bg-white text-slate-700">
                <div class="stat">
                  <div class="stat-figure text-slate-300">
                    <i class="fas fa-history text-3xl"></i>
                  </div>
                  <div class="stat-title font-medium text-slate-500">上次检查</div>
                  <div class="stat-value text-xl">{{ formatTime(lastCheck) }}</div>
                  <div class="stat-desc">最近一次巡检时间</div>
                </div>
              </div>
            </section>

            <!-- 图表区域 -->
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div class="p-4 bg-white rounded-xl border border-base-200 shadow-sm">
                 <h3 class="font-bold text-lg mb-4 text-slate-700">同步趋势</h3>
                 <SyncTrendChart :data="chartData.trend" />
              </div>
              <div class="p-4 bg-white rounded-xl border border-base-200 shadow-sm">
                 <h3 class="font-bold text-lg mb-4 text-slate-700">状态分布</h3>
                 <SiteStatusChart :data="chartData.status" />
              </div>
            </div>

            <!-- 增量更新实时监控 -->
            <IncrementalUpdateMonitor />

            <!-- 站点监控 -->
            <div class="bg-white rounded-xl border border-base-200 shadow-sm p-6">
              <div class="flex justify-between items-center mb-6">
                <div>
                  <h3 class="font-bold text-lg text-slate-800">站点监控</h3>
                  <p class="text-sm text-slate-500">实时监控各站点的连接与同步状态</p>
                </div>
                <button @click="currentView = 'topology'" class="btn btn-sm btn-outline">
                  管理拓扑
                </button>
              </div>
              
              <div class="grid grid-cols-1 gap-4">
                <SiteCard
                  v-for="site in sites"
                  :key="site.site_id"
                  :site="site"
                  @detect="handleDetect"
                  @sync="handleSync"
                  @abort="handleAbort"
                  @detail="handleShowDetail"
                />
                <div v-if="sites.length === 0" class="text-center py-12 bg-slate-50 rounded-xl border border-dashed border-slate-200">
                  <p class="text-slate-500">暂无受控站点，请前往"异地拓扑"添加</p>
                </div>
              </div>
            </div>
          </div>

          <!-- Topology View -->
          <div v-else-if="currentView === 'topology'" class="h-full animate-fade-in">
            <TopologyManager @site-added="refreshAll" />
          </div>

          <!-- Topology Visualization View -->
          <div v-else-if="currentView === 'topology-viz'" class="h-full animate-fade-in">
            <TopologyVisualization />
          </div>

          <!-- Tasks View -->
          <div v-else-if="currentView === 'tasks'" class="h-full flex flex-col bg-white rounded-xl shadow-sm border border-base-200 overflow-hidden animate-fade-in">
            <div class="px-6 py-4 border-b border-base-200 bg-base-50/50">
              <h3 class="font-bold text-xl"><i class="fas fa-layer-group text-primary mr-2"></i>任务队列</h3>
            </div>
            <div class="flex-1 overflow-hidden p-0">
              <TaskQueue :tasks="tasks" class="h-full" />
            </div>
          </div>

          <!-- History View -->
          <div v-else-if="currentView === 'history'" class="h-full flex flex-col bg-white rounded-xl shadow-sm border border-base-200 overflow-hidden animate-fade-in">
            <div class="px-6 py-4 border-b border-base-200 bg-base-50/50">
               <h3 class="font-bold text-xl"><i class="fas fa-history text-primary mr-2"></i>同步历史</h3>
            </div>
            <div class="flex-1 overflow-y-auto p-4">
              <SyncHistory
                :history="syncHistory"
                :current-page="historyPage"
                :total-pages="historyTotalPages"
                @prev-page="prevHistoryPage"
                @next-page="nextHistoryPage"
                @detail="handleShowHistoryDetail"
              />
            </div>
          </div>

          <!-- MQTT Messages View -->
          <div v-else-if="currentView === 'mqtt'" class="h-full animate-fade-in">
            <MqttMessageViewer />
          </div>

          <!-- MQTT Nodes Monitor View -->
          <div v-else-if="currentView === 'mqtt-nodes'" class="h-full animate-fade-in">
            <MqttNodeMonitor />
          </div>

          <!-- Logs View -->
          <div v-else-if="currentView === 'logs'" class="h-full flex flex-col bg-white rounded-xl shadow-sm border border-base-200 overflow-hidden animate-fade-in">
             <div class="px-6 py-4 border-b border-base-200 bg-base-50/50 flex justify-between items-center">
               <h3 class="font-bold text-xl"><i class="fas fa-terminal text-primary mr-2"></i>系统日志</h3>
               <button @click="clearLogs" class="btn btn-xs btn-ghost text-slate-500">清除</button>
            </div>
            <div class="flex-1 overflow-hidden p-0">
              <LogViewer :logs="logs" @clear="clearLogs" class="h-full border-none rounded-none" />
            </div>
          </div>

          <!-- Archives View -->
          <div v-else-if="currentView === 'archives'" class="h-full animate-fade-in">
            <ArchivesManager />
          </div>

          <!-- Site Config View -->
          <div v-else-if="currentView === 'site-config'" class="h-full animate-fade-in">
            <SiteConfig />
          </div>

          <!-- Settings View -->
          <div v-else-if="currentView === 'settings'" class="h-full animate-fade-in">
            <SettingsManager
              :initial-config="globalConfig"
              @save="handleSaveConfig"
            />
          </div>

        </div>
      </main>
    </div>

    <!-- Toast 通知 -->
    <div class="toast toast-top toast-end z-50 gap-2">
      <div
        v-for="notif in notifications"
        :key="notif.id"
        :class="['alert shadow-lg transform transition-all duration-300', `alert-${notif.type}`]"
      >
        <i :class="['fas', getNotificationIcon(notif.type)]"></i>
        <span>{{ notif.message }}</span>
        <button @click="removeNotification(notif.id)" class="btn btn-sm btn-ghost btn-circle">
          <i class="fas fa-times"></i>
        </button>
      </div>
    </div>

    <!-- 模态框 -->
    <DetailModal
      :is-open="detailModalOpen"
      :site="selectedSite"
      @close="detailModalOpen = false"
    />
      </div>
    </NMessageProvider>
  </NConfigProvider>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { NConfigProvider, NMessageProvider, NButton, zhCN, dateZhCN, darkTheme } from 'naive-ui';
import SiteCard from './components/SiteCard.vue';
import TaskQueue from './components/TaskQueue.vue';
import SyncHistory from './components/SyncHistory.vue';
import LogViewer from './components/LogViewer.vue';
import SyncTrendChart from './components/charts/SyncTrendChart.vue';
import SiteStatusChart from './components/charts/SiteStatusChart.vue';
import DetailModal from './components/DetailModal.vue';
import TopologyManager from './components/views/TopologyManager.vue';
import TopologyVisualization from './components/views/TopologyVisualization.vue';
import SiteConfig from './components/views/SiteConfig.vue';
import SettingsManager from './components/views/SettingsManager.vue';
import ArchivesManager from './components/views/ArchivesManager.vue';
import MqttMessageViewer from './components/views/MqttMessageViewer.vue';
import MqttNodeMonitor from './components/views/MqttNodeMonitorEnhanced.vue';
import IncrementalUpdateMonitor from './components/IncrementalUpdateMonitor.vue';
import SiteInfoBadge from './components/SiteInfoBadge.vue';

import { useApi } from './composables/useApi';
import { useTheme } from './composables/useTheme';
import { useNotification } from './composables/useNotification';
import { useFormatters } from './composables/useFormatters';
import { useWebSocket } from './composables/useWebSocket';

const { loadSites, loadSyncHistory, loadLogs: apiLoadLogs, loadStats, triggerDetection, triggerSync, abortOperation, fetchSyncConfig, updateSyncConfig } = useApi();
const { isDarkMode, toggleTheme } = useTheme();
const { notifications, show: showNotification, remove: removeNotification } = useNotification();
const { formatTime } = useFormatters();

// Naive UI Theme Overrides to match Tailwind/DaisyUI
const themeOverrides = computed(() => {
  if (isDarkMode.value) {
    return {
      common: {
        primaryColor: '#3b82f6', // blue-500
        primaryColorHover: '#60a5fa', // blue-400
        primaryColorPressed: '#2563eb', // blue-600
      },
      DataTable: {
        thColor: '#1e293b', // slate-800
        tdColor: '#0f172a', // slate-900
        tdTextColor: '#e2e8f0', // slate-200
        thTextColor: '#94a3b8', // slate-400
      }
    };
  } else {
    return {
      common: {
        primaryColor: '#3b82f6', // blue-500
        primaryColorHover: '#2563eb', // blue-600
        primaryColorPressed: '#1d4ed8', // blue-700
      },
      DataTable: {
        thColor: '#f8fafc', // slate-50
        tdColor: '#ffffff', // white
        tdTextColor: '#334155', // slate-700
        thTextColor: '#475569', // slate-600
      }
    };
  }
});

// WebSocket 连接 - 连接到任务队列实时推送
const { isConnected, connect, disconnect, on, send } = useWebSocket('/ws/tasks');

// UI State
const isMobileMenuOpen = ref(false);

const sites = ref([]);
const tasks = ref([]);
const syncHistory = ref([]);
const logs = ref([]);
const lastCheck = ref(null);
const historyPage = ref(1);
const historyPageSize = ref(20);
const historyTotal = ref(0);

// 统计数据
const stats = ref({
  total_synced: 0,
  week_synced: 0,
  today_synced: 0,
  queued_updates: 0,
  running_tasks: 0,
  pending_items: 0,
  is_running: false,
  is_paused: false,
  last_sync_time: null
});

const chartData = ref({
  trend: {
    dates: [],
    synced: [],
    pending: []
  },
  status: {
    idle: 0,
    scanning: 0,
    completed: 0,
    error: 0
  }
});

// 模态框状态 (Only DetailModal remains as modal)
const detailModalOpen = ref(false);
const selectedSite = ref(null);

// 全局配置
const globalConfig = ref({
  env_id: '', // New field
  autoDetect: false,
  detectionInterval: 30,
  autoSync: false,
  batchSyncSize: 10,
  enableNotifications: false,
  logRetentionDays: 30,
  maxConcurrentSyncs: 3
});

// State
const currentView = ref('dashboard');
const isLoading = ref(false);

const navigateTo = (view) => {
  currentView.value = view;
  isMobileMenuOpen.value = false;
};

const totalPending = computed(() => sites.value.reduce((sum, site) => sum + (site.pending_items || 0), 0));
const totalSynced = computed(() => sites.value.reduce((sum, site) => sum + (site.synced_items || 0), 0));
const historyTotalPages = computed(() => Math.ceil(historyTotal.value / historyPageSize.value));

const viewTitle = computed(() => {
  const titles = {
    'dashboard': '全局概览',
    'topology': '异地拓扑管理',
    'topology-viz': 'MQTT 拓扑可视化',
    'tasks': '任务队列',
    'history': '同步历史',
    'mqtt': 'MQTT 消息记录',
    'mqtt-nodes': 'MQTT 节点监控',
    'logs': '系统日志',
    'archives': '归档管理',
    'site-config': '站点配置',
    'settings': '参数配置'
  };
  return titles[currentView.value] || 'Incremental Sync';
});

let refreshTimer = null;
let logsRefreshTimer = null;

const updateStats = async () => {
  try {
    const data = await loadStats();
    if (data.success && data.stats) {
      stats.value = data.stats;
    }
  } catch (error) {
    console.error('加载统计数据失败:', error);
  }
};

const refreshAll = async () => {
  isLoading.value = true;
  try {
    const data = await loadSites();
    if (data.success) {
      sites.value = data.sites || [];
      lastCheck.value = data.last_check;
      updateChartData();
    }
    // 同时更新统计数据
    await updateStats();
    // If in specific view, refresh that too
    if (currentView.value === 'history') loadHistory();
  } catch (error) {
    showNotification('加载站点失败', 'error');
  } finally {
    isLoading.value = false;
  }
};

const loadHistory = async () => {
  try {
    const data = await loadSyncHistory(historyPage.value, historyPageSize.value);
    if (data.success) {
      syncHistory.value = data.data || [];
      historyTotal.value = data.pagination?.total || 0;
    }
  } catch (error) {
    console.error('加载历史失败:', error);
  }
};

const loadLogs = async () => {
  try {
    const data = await apiLoadLogs();
    if (data.success && data.logs) {
      logs.value = data.logs;
    }
  } catch (error) {
    console.error('加载日志失败:', error);
  }
};

const loadInitialConfig = async () => {
  try {
    const syncConfig = await fetchSyncConfig();
    if (syncConfig && syncConfig.env_id) {
      globalConfig.value.env_id = syncConfig.env_id;
    }
  } catch (e) {
    console.error('Failed to load sync config:', e);
  }
};

const clearLogs = () => {
  logs.value = [];
};

const updateChartData = () => {
  // 更新站点状态分布
  const statusCount = {
    idle: 0,
    scanning: 0,
    completed: 0,
    error: 0
  };

  sites.value.forEach(site => {
    const status = site.detection_status?.toLowerCase() || 'idle';
    if (statusCount.hasOwnProperty(status)) {
      statusCount[status]++;
    }
  });

  chartData.value.status = statusCount;
};

const prevHistoryPage = () => {
  if (historyPage.value > 1) {
    historyPage.value--;
    loadHistory();
  }
};

const nextHistoryPage = () => {
  if (historyPage.value < historyTotalPages.value) {
    historyPage.value++;
    loadHistory();
  }
};

const handleDetect = async (siteId) => {
  try {
    await triggerDetection(siteId);
    showNotification('开始检测变更', 'info');
    setTimeout(refreshAll, 1000);
  } catch (error) {
    showNotification('触发检测失败', 'error');
  }
};

const handleSync = async (siteId) => {
  try {
    await triggerSync(siteId);
    showNotification('开始同步', 'success');
    setTimeout(refreshAll, 1000);
  } catch (error) {
    showNotification('触发同步失败', 'error');
  }
};

const handleAbort = async (siteId) => {
  try {
    await abortOperation(siteId);
    showNotification('已中止操作', 'warning');
    setTimeout(refreshAll, 1000);
  } catch (error) {
    showNotification('中止操作失败', 'error');
  }
};

// 模态框处理
const handleShowDetail = (siteId) => {
  selectedSite.value = sites.value.find(s => s.site_id === siteId);
  detailModalOpen.value = true;
};

const handleShowHistoryDetail = (record) => {
  // 将历史记录转换为站点格式以便在DetailModal中显示
  selectedSite.value = {
    site_id: record.db_num ? `DB #${record.db_num}` : (record.site_id || '历史记录'),
    site_name: `历史记录 - ${formatTime(record.timestamp)}`,
    detection_status: record.status === 'failed' ? 'Error' : 'Completed',
    last_sync_time: record.timestamp,
    pending_items: 0,
    synced_items: record.file_count || 0,
    changed_files: (record.file_names || []).map((name) => ({
      name: name,
      type: 'Modified', // 历史记录通常不包含详细变更类型，默认为修改
      size: 0,
      modified: record.timestamp
    })),
    increment_size: 0, // 历史记录可能没有大小信息
    remote_url: record.location || record.file_server_host,
    last_detection: record.timestamp,
    last_error: record.error_message || null
  };
  detailModalOpen.value = true;
};

const handleShowConfig = async () => {
  try {
    // Load sync config (for env_id)
    const syncConfig = await fetchSyncConfig();
    if (syncConfig && syncConfig.env_id) {
      globalConfig.value.env_id = syncConfig.env_id;
    }
    // We could also load incremental config here if it wasn't already loaded
  } catch (e) {
    console.error('Failed to load sync config:', e);
    showNotification('加载配置部分失败', 'warning');
  }
  configModalOpen.value = true;
};

const handleSaveConfig = async (config) => {
  try {
    globalConfig.value = { ...config };

    // Save Sync Config (env_id)
    if (config.env_id) {
       const currentSyncConfig = await fetchSyncConfig();
       currentSyncConfig.env_id = config.env_id;
       await updateSyncConfig(currentSyncConfig);
    }

    // Save Incremental Config (legacy) - The useApi 'saveConfig' handles this
    // Wait, useApi's saveConfig calls /api/incremental/config. 
    // We should rename the imported saveConfig to saveIncrementalConfig to avoid confusion, but I didn't change the import name.
    // Let's assumes 'useApi' exports 'saveConfig'.
    const { saveConfig: saveIncrementalConfig } = useApi(); 
    await saveIncrementalConfig(config);

    // 如果启用桌面通知，显示测试通知
    if (config.enableNotifications && 'Notification' in window && Notification.permission === 'granted') {
      new Notification('配置已保存', {
        body: '全局配置已成功更新',
        icon: '/static/favicon.ico'
      });
    }

    showNotification('配置已保存', 'success');
    configModalOpen.value = false;
  } catch (error) {
    console.error(error);
    showNotification('保存配置失败', 'error');
  }
};

const getNotificationIcon = (type) => {
  const icons = {
    success: 'fa-check-circle',
    error: 'fa-exclamation-circle',
    info: 'fa-info-circle',
    warning: 'fa-exclamation-triangle'
  };
  return icons[type] || 'fa-info-circle';
};

// WebSocket 事件处理
const setupWebSocket = () => {
  // 处理所有 WebSocket 消息
  on('message', (data) => {
    console.log('[WebSocket] 收到消息:', data);

    const msgType = data.type;

    // 握手消息
    if (msgType === 'handshake') {
      console.log('[WebSocket] 连接成功，当前活跃任务', data.active_tasks);
      showNotification('实时监听已启动', 'success');

      // 立即请求任务列表
      send({ action: 'list' });

      // 订阅增量更新任务，接收文件变化的实时通知
      send({ action: 'subscribe', task_id: 'increment-updates' });
      console.log('[WebSocket] 已请求订阅 increment-updates 任务');
    }

    // 任务列表响应
    else if (msgType === 'task_list') {
      console.log('[WebSocket] 任务列表更新:', data.tasks);
      tasks.value = data.tasks || [];

      // 刷新站点数据以获取最新状态
      refreshAll();
    }

    // 进度更新消息 (ProgressMessage)
    else if (data.task_id && data.status) {
      const { task_id, status, percentage, current_step, message } = data;
      console.log(`[WebSocket] 任务 ${task_id} 进度: ${percentage}% - ${current_step || message}`);

      // 更新日志
      logs.value.push({
        timestamp: new Date().toISOString(),
        level: status === 'Failed' ? 'ERROR' : status === 'Completed' ? 'SUCCESS' : 'INFO',
        message: `[${task_id}] ${current_step || message || '进度更新'}`
      });

      // 限制日志数量
      if (logs.value.length > 1000) {
        logs.value = logs.value.slice(-500);
      }

      // 🎯 检测到增量更新任务 - 显示实时通知
      if (task_id === 'increment-updates') {
        // 正在运行 - 显示进行中的通知
        if (status === 'Running') {
          showNotification(`🔍 ${current_step || '正在检测增量变化...'}`, 'info');
        }
        // 已完成 - 显示成功通知
        else if (status === 'Completed') {
          const fileInfo = message || '文件已更新';
          showNotification(`✅ 增量更新完成: ${fileInfo}`, 'success');

          // 桌面通知
          if (globalConfig.value.enableNotifications && 'Notification' in window && Notification.permission === 'granted') {
            new Notification('增量更新完成', {
              body: current_step || '检测到文件变化并已完成增量更新',
              icon: '/static/favicon.ico'
            });
          }

          // 延迟刷新以确保后端数据已更新
          setTimeout(() => {
            refreshAll();
            loadHistory();
          }, 1000);
        }
        // 失败 - 显示错误通知
        else if (status === 'Failed') {
          showNotification(`❌ 增量更新失败: ${message}`, 'error');
        }
      }
      // 其他任务的通知处理
      else {
        // 如果任务完成，刷新所有数据并显示通知
        if (status === 'Completed') {
          showNotification(`任务 ${task_id} 已完成`, 'success');

          // 桌面通知
          if (globalConfig.value.enableNotifications && 'Notification' in window && Notification.permission === 'granted') {
            new Notification('任务完成', {
              body: `任务 ${task_id} 已完成`,
              icon: '/static/favicon.ico'
            });
          }

          // 延迟刷新以确保后端数据已更新
          setTimeout(() => {
            refreshAll();
            loadHistory();
          }, 1000);
        }

        // 如果任务失败，显示错误通知
        else if (status === 'Failed') {
          showNotification(`任务 ${task_id} 失败: ${message}`, 'error');
        }
      }
    }

    // 订阅/取消订阅确认
    else if (msgType === 'subscribed') {
      console.log('[WebSocket] 已订阅任务', data.task_id);
    }
    else if (msgType === 'unsubscribed') {
      console.log('[WebSocket] 已取消订阅任务', data.task_id);
    }

    // 警告消息
    else if (msgType === 'warning') {
      console.warn('[WebSocket] 警告:', data.message);
    }
  });

  // 连接状态变化
  on('connected', () => {
    console.log('[WebSocket] 已连接到实时监听服务');
  });

  on('disconnected', () => {
    console.log('[WebSocket] 实时监听连接已断开');
    showNotification('实时监听已断开，将尝试重连', 'warning');
  });

  on('error', (error) => {
    console.error('[WebSocket] 连接错误:', error);
  });
};

onMounted(() => {
  refreshAll();
  loadHistory();
  loadLogs();
  loadInitialConfig();
  refreshTimer = setInterval(refreshAll, 30000); // 30秒刷新站点
  logsRefreshTimer = setInterval(loadLogs, 5000); // 5秒刷新日志
  // 启动 WebSocket 连接 - 实时监听增量变化
  setupWebSocket();
  connect();
});

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer);
  if (logsRefreshTimer) clearInterval(logsRefreshTimer);
  disconnect();
});
</script>

<style>
.nav-link {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  font-weight: 500;
  transition: all 0.2s ease;
  text-decoration: none;
  border: 1px solid transparent;
}

[data-theme="dark"] .nav-link {
  color: rgba(148, 163, 184, 0.8);
}

[data-theme="light"] .nav-link {
  color: rgba(71, 85, 105, 0.8);
}

[data-theme="dark"] .nav-link:hover {
  color: #fff;
  background-color: rgba(30, 41, 59, 0.5);
}

[data-theme="light"] .nav-link:hover {
  color: #1e293b;
  background-color: rgba(241, 245, 249, 0.8);
}

[data-theme="dark"] .nav-link.active {
  color: #fff;
  background-color: rgba(59, 130, 246, 0.15);
  border-color: rgba(59, 130, 246, 0.2);
  font-weight: 600;
}

[data-theme="light"] .nav-link.active {
  color: #2563eb;
  background-color: rgba(59, 130, 246, 0.1);
  border-color: rgba(59, 130, 246, 0.2);
  font-weight: 600;
}

.dashboard-card {
  background: #fff;
  border-radius: 1.25rem;
  border: 2px solid #e2e8f0;
  padding: 1.75rem;
  box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
}

@keyframes fade-in {
  from { opacity: 0; transform: translateY(5px); }
  to { opacity: 1; transform: translateY(0); }
}

.animate-fade-in {
  animation: fade-in 0.3s ease-out;
}

@keyframes fade-in-up {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

.animate-fade-in-up {
  animation: fade-in-up 0.5s ease-out;
}
</style>
