import { ref } from 'vue';

export function useApi() {
  const loading = ref(false);
  const error = ref(null);

  async function fetchData(url, options = {}) {
    loading.value = true;
    error.value = null;

    try {
      const response = await fetch(url, options);
      const data = await response.json();
      return data;
    } catch (err) {
      error.value = err.message;
      console.error('API Error:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  }

  async function loadSites() {
    return fetchData('/api/incremental/status');
  }

  async function loadSyncHistory(page = 1, pageSize = 20) {
    return fetchData(`/api/incremental/history?page=${page}&page_size=${pageSize}`);
  }

  async function loadConfig() {
    return fetchData('/api/incremental/config');
  }

  async function loadLogs() {
    return fetchData('/api/incremental/logs');
  }

  async function loadArchives() {
    return fetchData('/api/incremental/archives');
  }

  async function loadStats() {
    return fetchData('/api/incremental/stats');
  }

  async function triggerDetection(siteId) {
    return fetchData(`/api/incremental/detect/${siteId}`, { method: 'POST' });
  }

  async function triggerSync(siteId) {
    return fetchData(`/api/incremental/sync/${siteId}`, { method: 'POST' });
  }

  async function abortOperation(siteId) {
    return fetchData(`/api/incremental/abort/${siteId}`, { method: 'POST' });
  }

  async function saveConfig(config) {
    return fetchData('/api/incremental/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config)
    });
  }

  // Remote Sync Environment Management
  async function fetchRemoteEnvs() {
    return fetchData('/api/remote-sync/envs');
  }

  async function createRemoteEnv(env) {
    return fetchData('/api/remote-sync/envs', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(env)
    });
  }

  async function deleteRemoteEnv(envId) {
    return fetchData(`/api/remote-sync/envs/${envId}`, {
      method: 'DELETE'
    });
  }

  // Remote Sync Site Management (Nested under Env)
  async function fetchRemoteSites(envId) {
    return fetchData(`/api/remote-sync/envs/${envId}/sites`);
  }

  async function createRemoteSite(envId, site) {
    return fetchData(`/api/remote-sync/envs/${envId}/sites`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(site)
    });
  }

  async function deleteRemoteSite(siteId) {
    return fetchData(`/api/remote-sync/sites/${siteId}`, {
      method: 'DELETE'
    });
  }

  // Remote Sync Config (Env ID)
  async function fetchSyncConfig() {
    return fetchData('/api/sync/config');
  }

  async function updateSyncConfig(config) {
    return fetchData('/api/sync/config', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config)
    });
  }

  // Dashboard API - 增量更新实时监控
  async function fetchDashboardSummary() {
    return fetchData('/api/dashboard/summary');
  }

  async function fetchActiveTasks() {
    return fetchData('/api/dashboard/active-tasks');
  }

  async function fetchFailedTasks(filters = {}) {
    const params = new URLSearchParams();
    if (filters.task_type) params.append('task_type', filters.task_type);
    if (filters.pending_only) params.append('pending_only', 'true');
    if (filters.exhausted_only) params.append('exhausted_only', 'true');
    const query = params.toString();
    return fetchData(`/api/dashboard/failed-tasks${query ? '?' + query : ''}`);
  }

  async function fetchTimelineStats(window = '24h') {
    return fetchData(`/api/dashboard/stats/timeline?window=${window}`);
  }

  async function retryFailedTask(taskId) {
    return fetchData(`/api/dashboard/retry-task/${taskId}`, {
      method: 'POST'
    });
  }

  async function cleanupExhaustedTasks() {
    return fetchData('/api/dashboard/cleanup-exhausted', {
      method: 'POST'
    });
  }

  async function fetchIncrementElements(syncId, filters = {}) {
    const params = new URLSearchParams();
    if (filters.operation) params.append('operation', filters.operation);
    if (filters.limit) params.append('limit', filters.limit);
    if (filters.offset) params.append('offset', filters.offset);
    const query = params.toString();
    return fetchData(`/api/dashboard/increment-elements/${syncId}${query ? '?' + query : ''}`);
  }

  return {
    loading,
    error,
    loadSites,
    loadSyncHistory,
    loadConfig,
    loadLogs,
    loadArchives,
    loadStats,
    triggerDetection,
    triggerSync,
    abortOperation,
    saveConfig,
    // New exports
    fetchRemoteEnvs,
    createRemoteEnv,
    deleteRemoteEnv,
    fetchRemoteSites,
    createRemoteSite,
    deleteRemoteSite,
    fetchSyncConfig,
    updateSyncConfig,
    // Dashboard exports
    fetchDashboardSummary,
    fetchActiveTasks,
    fetchFailedTasks,
    fetchTimelineStats,
    retryFailedTask,
    cleanupExhaustedTasks,
    fetchIncrementElements
  };
}
