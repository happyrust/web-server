import { ref } from 'vue';

export function useNotification() {
  const notifications = ref([]);
  let nextId = 0;

  const show = (message, type = 'info') => {
    const id = nextId++;
    notifications.value.push({ id, message, type });

    setTimeout(() => {
      remove(id);
    }, 3000);
  };

  const remove = (id) => {
    const index = notifications.value.findIndex(n => n.id === id);
    if (index !== -1) {
      notifications.value.splice(index, 1);
    }
  };

  const success = (message) => show(message, 'success');
  const error = (message) => show(message, 'error');
  const info = (message) => show(message, 'info');
  const warning = (message) => show(message, 'warning');

  return {
    notifications,
    show,
    remove,
    success,
    error,
    info,
    warning
  };
}
