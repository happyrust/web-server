/// 增量更新实时监控仪表盘页面

pub fn render_dashboard_page() -> String {
    let content = r#"
<div x-data="dashboardApp()" x-init="init()" x-cloak class="space-y-6">
  <!-- Toast 通知 -->
  <div x-show="toast.show" x-transition class="fixed top-20 right-6 z-50" style="display:none;">
    <div :class="toastClass()" class="flex items-center space-x-2 px-4 py-2 rounded-lg shadow-lg">
      <i :class="toastIcon()"></i>
      <span class="text-sm" x-text="toast.text"></span>
    </div>
  </div>

  <!-- 页面标题 -->
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold text-gray-800">
      <i class="fas fa-chart-line mr-2 text-blue-600"></i>增量更新实时监控
    </h1>
    <div class="space-x-2">
      <button @click="refreshAll()" class="px-3 py-1.5 bg-gray-100 rounded hover:bg-gray-200">
        <i class="fas fa-sync mr-1" :class="{'fa-spin': loading}"></i> 刷新
      </button>
    </div>
  </div>

  <!-- 统计卡片 -->
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
    <!-- 进行中任务 -->
    <div class="bg-white rounded-lg shadow p-4">
      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm text-gray-600">进行中</p>
          <p class="text-2xl font-bold text-blue-600" x-text="summary.task_stats?.in_progress || 0"></p>
        </div>
        <i class="fas fa-spinner text-3xl text-blue-200"></i>
      </div>
    </div>

    <!-- 今日完成 -->
    <div class="bg-white rounded-lg shadow p-4">
      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm text-gray-600">今日完成</p>
          <p class="text-2xl font-bold text-green-600" x-text="summary.task_stats?.completed_today || 0"></p>
        </div>
        <i class="fas fa-check-circle text-3xl text-green-200"></i>
      </div>
    </div>

    <!-- 今日失败 -->
    <div class="bg-white rounded-lg shadow p-4">
      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm text-gray-600">今日失败</p>
          <p class="text-2xl font-bold text-red-600" x-text="summary.task_stats?.failed_today || 0"></p>
        </div>
        <i class="fas fa-exclamation-circle text-3xl text-red-200"></i>
      </div>
    </div>

    <!-- 成功率 -->
    <div class="bg-white rounded-lg shadow p-4">
      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm text-gray-600">成功率</p>
          <p class="text-2xl font-bold text-purple-600" x-text="(summary.performance_metrics?.success_rate || 0).toFixed(1) + '%'"></p>
        </div>
        <i class="fas fa-chart-pie text-3xl text-purple-200"></i>
      </div>
    </div>
  </div>

  <!-- 失败任务统计 -->
  <div class="bg-white rounded-lg shadow p-4">
    <h2 class="font-semibold text-gray-700 mb-3">
      <i class="fas fa-exclamation-triangle mr-2 text-amber-500"></i>失败任务队列
    </h2>
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <div class="text-center">
        <p class="text-sm text-gray-600">待重试</p>
        <p class="text-xl font-bold text-amber-600" x-text="summary.failed_task_stats?.pending_retry || 0"></p>
      </div>
      <div class="text-center">
        <p class="text-sm text-gray-600">等待中</p>
        <p class="text-xl font-bold text-gray-600" x-text="summary.failed_task_stats?.waiting || 0"></p>
      </div>
      <div class="text-center">
        <p class="text-sm text-gray-600">已耗尽</p>
        <p class="text-xl font-bold text-red-600" x-text="summary.failed_task_stats?.exhausted || 0"></p>
      </div>
      <div class="text-center">
        <p class="text-sm text-gray-600">总计</p>
        <p class="text-xl font-bold text-gray-800" x-text="summary.failed_task_stats?.total || 0"></p>
      </div>
    </div>
    <div class="mt-3 flex justify-end space-x-2">
      <button @click="showFailedTasks()" class="px-3 py-1.5 bg-amber-100 text-amber-700 rounded hover:bg-amber-200">
        <i class="fas fa-list mr-1"></i> 查看详情
      </button>
      <button @click="cleanupExhausted()" class="px-3 py-1.5 bg-red-100 text-red-700 rounded hover:bg-red-200">
        <i class="fas fa-trash mr-1"></i> 清理已耗尽
      </button>
    </div>
  </div>

  <!-- 活跃任务列表 -->
  <div class="bg-white rounded-lg shadow p-4">
    <h2 class="font-semibold text-gray-700 mb-3">
      <i class="fas fa-tasks mr-2 text-blue-500"></i>活跃任务
    </h2>
    <div class="space-y-3">
      <template x-if="activeTasks.length === 0">
        <p class="text-gray-500 text-sm">当前没有活跃任务</p>
      </template>
      <template x-for="task in activeTasks" :key="task.task_id">
        <div class="border rounded-lg p-3 hover:bg-gray-50 transition">
          <div class="flex items-center justify-between mb-2">
            <div class="flex-1">
              <div class="font-medium text-gray-800" x-text="task.task_name"></div>
              <div class="text-xs text-gray-500" x-text="task.file_path"></div>
            </div>
            <span class="px-2 py-1 text-xs rounded"
                  :class="getStatusClass(task.status)"
                  x-text="task.status"></span>
          </div>
          <div class="flex items-center space-x-2">
            <div class="flex-1 bg-gray-200 rounded-full h-2">
              <div class="bg-blue-600 h-2 rounded-full transition-all"
                   :style="`width: ${task.progress}%`"></div>
            </div>
            <span class="text-xs text-gray-600" x-text="task.progress.toFixed(0) + '%'"></span>
          </div>
        </div>
      </template>
    </div>
  </div>

  <!-- 趋势图表 -->
  <div class="bg-white rounded-lg shadow p-4">
    <div class="flex items-center justify-between mb-3">
      <h2 class="font-semibold text-gray-700">
        <i class="fas fa-chart-area mr-2 text-purple-500"></i>任务趋势
      </h2>
      <select x-model="selectedTimeWindow" @change="loadTimelineData()" class="px-3 py-1 border rounded text-sm">
        <option value="1h">最近1小时</option>
        <option value="24h" selected>最近24小时</option>
        <option value="7d">最近7天</option>
        <option value="30d">最近30天</option>
      </select>
    </div>
    <div style="height: 300px;">
      <canvas id="taskTrendChart"></canvas>
    </div>
  </div>

  <!-- 成功率和平均耗时 -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
    <!-- 成功率图表 -->
    <div class="bg-white rounded-lg shadow p-4">
      <h2 class="font-semibold text-gray-700 mb-3">
        <i class="fas fa-percentage mr-2 text-green-500"></i>成功率趋势
      </h2>
      <div style="height: 200px;">
        <canvas id="successRateChart"></canvas>
      </div>
    </div>

    <!-- 平均耗时图表 -->
    <div class="bg-white rounded-lg shadow p-4">
      <h2 class="font-semibold text-gray-700 mb-3">
        <i class="fas fa-clock mr-2 text-blue-500"></i>平均耗时
      </h2>
      <div style="height: 200px;">
        <canvas id="avgDurationChart"></canvas>
      </div>
    </div>
  </div>

  <!-- 最近事件 -->
  <div class="bg-white rounded-lg shadow p-4">
    <h2 class="font-semibold text-gray-700 mb-3">
      <i class="fas fa-history mr-2 text-green-500"></i>最近事件
    </h2>
    <div class="space-y-2">
      <template x-if="recentEvents.length === 0">
        <p class="text-gray-500 text-sm">暂无最近事件</p>
      </template>
      <template x-for="event in recentEvents" :key="event.timestamp">
        <div class="flex items-start space-x-3 text-sm">
          <i :class="getEventIcon(event.event_type)" class="mt-1"></i>
          <div class="flex-1">
            <p class="text-gray-700" x-text="event.message"></p>
            <p class="text-xs text-gray-500" x-text="formatTime(event.timestamp)"></p>
          </div>
        </div>
      </template>
    </div>
  </div>

  <!-- 失败任务详情模态框 -->
  <div x-show="showFailedModal" x-cloak
       class="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50"
       @click.self="showFailedModal = false">
    <div class="bg-white rounded-lg shadow-xl p-6 w-full max-w-4xl max-h-[80vh] overflow-y-auto">
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-xl font-semibold">失败任务详情</h3>
        <button @click="showFailedModal = false" class="text-gray-500 hover:text-gray-700">
          <i class="fas fa-times text-xl"></i>
        </button>
      </div>

      <!-- 过滤器 -->
      <div class="flex space-x-2 mb-4">
        <select x-model="failedFilter.type" @change="loadFailedTasks()" class="px-3 py-2 border rounded">
          <option value="">所有类型</option>
          <option value="database_query">数据库查询</option>
          <option value="compression">CBA压缩</option>
          <option value="increment_update">增量更新</option>
          <option value="mqtt_publish">MQTT推送</option>
        </select>
        <select x-model="failedFilter.status" @change="loadFailedTasks()" class="px-3 py-2 border rounded">
          <option value="">所有状态</option>
          <option value="pending">待重试</option>
          <option value="exhausted">已耗尽</option>
        </select>
      </div>

      <!-- 失败任务列表 -->
      <div class="space-y-3">
        <template x-if="failedTasks.length === 0">
          <p class="text-gray-500 text-center py-8">没有失败任务</p>
        </template>
        <template x-for="task in failedTasks" :key="task.id">
          <div class="border rounded-lg p-4 hover:bg-gray-50">
            <div class="flex items-start justify-between mb-2">
              <div class="flex-1">
                <div class="font-medium text-gray-800" x-text="getTaskTypeLabel(task.task_type)"></div>
                <div class="text-sm text-red-600 mt-1" x-text="task.error"></div>
              </div>
              <div class="flex items-center space-x-2">
                <span class="px-2 py-1 text-xs rounded bg-amber-100 text-amber-700"
                      x-text="`${task.retry_count}/${task.max_retries}`"></span>
                <button @click="retryTask(task.id)" class="px-3 py-1 text-sm bg-blue-100 text-blue-700 rounded hover:bg-blue-200">
                  <i class="fas fa-redo mr-1"></i>重试
                </button>
              </div>
            </div>
            <div class="text-xs text-gray-500 space-y-1">
              <div>首次失败: <span x-text="formatTime(task.first_failed_at * 1000)"></span></div>
              <div x-show="task.next_retry_at">
                下次重试: <span x-text="formatTime(task.next_retry_at * 1000)"></span>
              </div>
              <div x-show="task.metadata" class="mt-2">
                <details class="cursor-pointer">
                  <summary class="text-blue-600">查看元数据</summary>
                  <pre class="mt-2 p-2 bg-gray-100 rounded text-xs overflow-x-auto" x-text="JSON.stringify(task.metadata, null, 2)"></pre>
                </details>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</div>

<style>
[x-cloak] { display: none !important; }
</style>

<script src="/static/dashboard.js"></script>
"#;

    crate::web_server::layout::render_layout_with_sidebar(
        "增量更新监控",
        Some("dashboard"),
        content,
        Some(
            "<link rel=\"stylesheet\" href=\"/static/dashboard.css\">\n<script src=\"/static/chart.umd.min.js\"></script>",
        ),
        None,
    )
}
