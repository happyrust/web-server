// 增量更新实时监控仪表盘 - Alpine.js
function dashboardApp() {
    return {
        // 数据状态
        summary: {},
        activeTasks: [],
        recentEvents: [],
        loading: false,
        selectedTimeWindow: '24h',

        // 失败任务模态框
        showFailedModal: false,
        failedTasks: [],
        failedFilter: {
            type: '',
            status: ''
        },

        // Chart.js实例
        charts: {
            taskTrend: null,
            successRate: null,
            avgDuration: null
        },

        // Toast通知
        toast: {
            show: false,
            text: '',
            type: 'info' // info, success, error, warning
        },

        // 初始化
        async init() {
            await this.loadSummary();
            await this.loadActiveTasks();
            await this.initCharts();
            await this.loadTimelineData();
            this.startPolling();
            this.connectWebSocket();
        },

        // 加载概览统计
        async loadSummary() {
            try {
                const response = await fetch('/api/dashboard/summary');
                if (response.ok) {
                    this.summary = await response.json();
                }
            } catch (error) {
                console.error('加载统计失败:', error);
            }
        },

        // 加载活跃任务
        async loadActiveTasks() {
            try {
                const response = await fetch('/api/dashboard/active-tasks');
                if (response.ok) {
                    this.activeTasks = await response.json();
                }
            } catch (error) {
                console.error('加载活跃任务失败:', error);
            }
        },

        // 刷新所有数据
        async refreshAll() {
            this.loading = true;
            await Promise.all([
                this.loadSummary(),
                this.loadActiveTasks()
            ]);
            this.loading = false;
            this.showToast('数据已刷新', 'success');
        },

        // 定时轮询（30秒）
        startPolling() {
            setInterval(async () => {
                await this.loadSummary();
                await this.loadActiveTasks();
            }, 30000);
        },

        // WebSocket连接（实时推送）
        connectWebSocket() {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const ws = new WebSocket(`${protocol}//${window.location.host}/ws/tasks`);

            ws.onopen = () => {
                console.log('WebSocket连接已建立');
            };

            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    this.handleWebSocketMessage(data);
                } catch (error) {
                    console.error('解析WebSocket消息失败:', error);
                }
            };

            ws.onerror = (error) => {
                console.error('WebSocket错误:', error);
            };

            ws.onclose = () => {
                console.log('WebSocket连接已关闭,5秒后重连');
                setTimeout(() => this.connectWebSocket(), 5000);
            };
        },

        // 处理WebSocket消息
        handleWebSocketMessage(data) {
            switch (data.type) {
                case 'TaskStatusChange':
                    this.updateTaskStatus(data.task_id, data.status, data.progress);
                    break;
                case 'StatsUpdate':
                    this.summary = data.stats;
                    break;
                case 'NewFailedTask':
                    this.loadSummary(); // 重新加载统计
                    this.showToast('新增失败任务', 'warning');
                    break;
            }
        },

        // 更新任务状态
        updateTaskStatus(taskId, status, progress) {
            const task = this.activeTasks.find(t => t.task_id === taskId);
            if (task) {
                task.status = status;
                if (progress !== undefined) {
                    task.progress = progress;
                }
            }
        },

        // 初始化Chart.js图表
        async initCharts() {
            // 确保Chart.js已加载
            if (typeof Chart === 'undefined') {
                console.error('Chart.js未加载');
                return;
            }

            Chart.defaults.font.family = 'system-ui, -apple-system, sans-serif';
            Chart.defaults.color = '#6b7280';

            // 任务趋势图（面积图）
            const ctx1 = document.getElementById('taskTrendChart');
            if (ctx1) {
                this.charts.taskTrend = new Chart(ctx1, {
                    type: 'line',
                    data: {
                        labels: [],
                        datasets: [
                            {
                                label: '成功',
                                data: [],
                                borderColor: '#10b981',
                                backgroundColor: 'rgba(16, 185, 129, 0.1)',
                                fill: true,
                                tension: 0.4
                            },
                            {
                                label: '失败',
                                data: [],
                                borderColor: '#ef4444',
                                backgroundColor: 'rgba(239, 68, 68, 0.1)',
                                fill: true,
                                tension: 0.4
                            }
                        ]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { position: 'top' },
                            tooltip: { mode: 'index', intersect: false }
                        },
                        scales: {
                            y: { beginAtZero: true, ticks: { stepSize: 1 } }
                        }
                    }
                });
            }

            // 成功率图表（折线图）
            const ctx2 = document.getElementById('successRateChart');
            if (ctx2) {
                this.charts.successRate = new Chart(ctx2, {
                    type: 'line',
                    data: {
                        labels: [],
                        datasets: [{
                            label: '成功率(%)',
                            data: [],
                            borderColor: '#3b82f6',
                            backgroundColor: 'rgba(59, 130, 246, 0.1)',
                            fill: true,
                            tension: 0.4
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { display: false }
                        },
                        scales: {
                            y: { beginAtZero: true, max: 100, ticks: { callback: v => v + '%' } }
                        }
                    }
                });
            }

            // 平均耗时图表（柱状图）
            const ctx3 = document.getElementById('avgDurationChart');
            if (ctx3) {
                this.charts.avgDuration = new Chart(ctx3, {
                    type: 'bar',
                    data: {
                        labels: [],
                        datasets: [{
                            label: '平均耗时(秒)',
                            data: [],
                            backgroundColor: '#8b5cf6'
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { display: false }
                        },
                        scales: {
                            y: { beginAtZero: true }
                        }
                    }
                });
            }
        },

        // 加载时间线数据
        async loadTimelineData() {
            try {
                const response = await fetch(`/api/dashboard/stats/timeline?window=${this.selectedTimeWindow}`);
                if (response.ok) {
                    const data = await response.json();
                    this.updateCharts(data);
                }
            } catch (error) {
                console.error('加载时间线数据失败:', error);
            }
        },

        // 更新图表数据
        updateCharts(timelineData) {
            if (!timelineData || timelineData.length === 0) {
                return;
            }

            const labels = timelineData.map(d => {
                const date = new Date(d.timestamp);
                return this.selectedTimeWindow === '1h'
                    ? date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
                    : date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' });
            });

            const successData = timelineData.map(d => d.success_count);
            const failureData = timelineData.map(d => d.failure_count);
            const successRateData = timelineData.map(d => {
                const total = d.success_count + d.failure_count;
                return total > 0 ? (d.success_count / total * 100).toFixed(1) : 0;
            });
            const avgDurationData = timelineData.map(d => d.avg_duration_secs.toFixed(2));

            // 更新任务趋势图
            if (this.charts.taskTrend) {
                this.charts.taskTrend.data.labels = labels;
                this.charts.taskTrend.data.datasets[0].data = successData;
                this.charts.taskTrend.data.datasets[1].data = failureData;
                this.charts.taskTrend.update();
            }

            // 更新成功率图
            if (this.charts.successRate) {
                this.charts.successRate.data.labels = labels;
                this.charts.successRate.data.datasets[0].data = successRateData;
                this.charts.successRate.update();
            }

            // 更新平均耗时图
            if (this.charts.avgDuration) {
                this.charts.avgDuration.data.labels = labels;
                this.charts.avgDuration.data.datasets[0].data = avgDurationData;
                this.charts.avgDuration.update();
            }
        },

        // 显示失败任务详情
        async showFailedTasks() {
            this.showFailedModal = true;
            await this.loadFailedTasks();
        },

        // 加载失败任务列表
        async loadFailedTasks() {
            try {
                let url = '/api/dashboard/failed-tasks?';
                if (this.failedFilter.type) {
                    url += `task_type=${this.failedFilter.type}&`;
                }
                if (this.failedFilter.status === 'pending') {
                    url += 'pending_only=true&';
                } else if (this.failedFilter.status === 'exhausted') {
                    url += 'exhausted_only=true&';
                }

                const response = await fetch(url);
                if (response.ok) {
                    this.failedTasks = await response.json();
                }
            } catch (error) {
                console.error('加载失败任务失败:', error);
            }
        },

        // 手动重试任务
        async retryTask(taskId) {
            try {
                const response = await fetch(`/api/dashboard/retry-task/${taskId}`, {
                    method: 'POST'
                });

                if (response.ok) {
                    this.showToast('任务已加入重试队列', 'success');
                    await this.loadFailedTasks();
                    await this.loadSummary();
                } else {
                    this.showToast('重试失败', 'error');
                }
            } catch (error) {
                console.error('重试任务失败:', error);
                this.showToast('请求失败', 'error');
            }
        },

        // 获取任务类型标签
        getTaskTypeLabel(taskType) {
            const typeKey = taskType.type || Object.keys(taskType)[0];
            const labels = {
                'DatabaseQuery': '数据库查询失败',
                'Compression': 'CBA压缩失败',
                'IncrementUpdate': '增量更新失败',
                'MqttPublish': 'MQTT推送失败'
            };
            return labels[typeKey] || '未知类型';
        },

        // 清理已耗尽任务
        async cleanupExhausted() {
            if (!confirm('确定要清理所有已耗尽的失败任务吗？')) {
                return;
            }

            try {
                const response = await fetch('/api/dashboard/cleanup-exhausted', {
                    method: 'POST'
                });

                if (response.ok) {
                    const result = await response.json();
                    this.showToast(result.message, 'success');
                    await this.loadSummary();
                } else {
                    this.showToast('清理失败', 'error');
                }
            } catch (error) {
                console.error('清理失败:', error);
                this.showToast('请求失败', 'error');
            }
        },

        // 显示Toast通知
        showToast(text, type = 'info') {
            this.toast = { show: true, text, type };
            setTimeout(() => {
                this.toast.show = false;
            }, 3000);
        },

        // Toast样式类
        toastClass() {
            const classes = {
                info: 'bg-blue-100 text-blue-800',
                success: 'bg-green-100 text-green-800',
                error: 'bg-red-100 text-red-800',
                warning: 'bg-amber-100 text-amber-800'
            };
            return classes[this.toast.type] || classes.info;
        },

        // Toast图标
        toastIcon() {
            const icons = {
                info: 'fas fa-info-circle text-blue-600',
                success: 'fas fa-check-circle text-green-600',
                error: 'fas fa-times-circle text-red-600',
                warning: 'fas fa-exclamation-triangle text-amber-600'
            };
            return icons[this.toast.type] || icons.info;
        },

        // 获取任务状态样式
        getStatusClass(status) {
            const classes = {
                'Pending': 'bg-gray-100 text-gray-700',
                'Running': 'bg-blue-100 text-blue-700',
                'Completed': 'bg-green-100 text-green-700',
                'Failed': 'bg-red-100 text-red-700',
                'Cancelled': 'bg-gray-100 text-gray-600'
            };
            return classes[status] || 'bg-gray-100 text-gray-700';
        },

        // 获取事件图标
        getEventIcon(eventType) {
            const icons = {
                'task_started': 'fas fa-play-circle text-blue-500',
                'task_completed': 'fas fa-check-circle text-green-500',
                'task_failed': 'fas fa-times-circle text-red-500',
                'system_alert': 'fas fa-exclamation-triangle text-amber-500',
                'retry_success': 'fas fa-redo text-green-500'
            };
            return icons[eventType] || 'fas fa-circle text-gray-500';
        },

        // 格式化时间
        formatTime(timestamp) {
            const date = new Date(timestamp);
            const now = new Date();
            const diff = (now - date) / 1000; // 秒

            if (diff < 60) {
                return '刚刚';
            } else if (diff < 3600) {
                return `${Math.floor(diff / 60)}分钟前`;
            } else if (diff < 86400) {
                return `${Math.floor(diff / 3600)}小时前`;
            } else {
                return date.toLocaleString('zh-CN');
            }
        }
    };
}
