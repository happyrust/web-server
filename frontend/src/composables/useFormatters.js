export function useFormatters() {
  const formatTime = (time) => {
    if (!time) return '--';
    const date = new Date(time);
    return date.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  const formatSize = (bytes) => {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
  };

  const formatDuration = (seconds) => {
    if (!seconds) return '--';
    if (seconds < 60) return `${seconds} 秒`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟`;
    return `${Math.floor(seconds / 3600)} 小时 ${Math.floor((seconds % 3600) / 60)} 分钟`;
  };

  const getStatusClass = (status) => {
    const statusMap = {
      'Idle': 'status-idle',
      'Scanning': 'status-scanning',
      'ChangesDetected': 'status-changes-detected',
      'Syncing': 'status-syncing',
      'Completed': 'status-completed',
      'Error': 'status-error'
    };
    return statusMap[status] || 'status-idle';
  };

  const getStatusText = (status) => {
    const textMap = {
      'Idle': '空闲',
      'Scanning': '扫描中',
      'ChangesDetected': '发现变更',
      'Syncing': '同步中',
      'Completed': '已完成',
      'Error': '错误'
    };
    return textMap[status] || status;
  };

  const getStatusIcon = (status) => {
    const iconMap = {
      'Idle': 'fas fa-circle',
      'Scanning': 'fas fa-spinner fa-spin',
      'ChangesDetected': 'fas fa-exclamation-triangle',
      'Syncing': 'fas fa-sync fa-spin',
      'Completed': 'fas fa-check-circle',
      'Error': 'fas fa-times-circle'
    };
    return iconMap[status] || 'fas fa-circle';
  };

  const getChangeTypeClass = (type) => {
    const typeMap = {
      'Added': 'change-added',
      'Modified': 'change-modified',
      'Deleted': 'change-deleted'
    };
    return typeMap[type] || '';
  };

  const getChangeTypeLabel = (type) => {
    const labelMap = {
      'Added': '新增',
      'Modified': '修改',
      'Deleted': '删除'
    };
    return labelMap[type] || type;
  };

  const getChangeTypeIcon = (type) => {
    const iconMap = {
      'Added': '➕',
      'Modified': '✏️',
      'Deleted': '🗑️'
    };
    return iconMap[type] || '';
  };

  return {
    formatTime,
    formatSize,
    formatDuration,
    getStatusClass,
    getStatusText,
    getStatusIcon,
    getChangeTypeClass,
    getChangeTypeLabel,
    getChangeTypeIcon
  };
}
