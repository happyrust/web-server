import { ref, onMounted, onUnmounted } from 'vue';

export function useWebSocket(url) {
  const ws = ref(null);
  const isConnected = ref(false);
  const reconnectAttempts = ref(0);
  const maxReconnectAttempts = 5;
  const reconnectInterval = ref(null);
  const messageHandlers = new Map();

  const connect = () => {
    if (ws.value && ws.value.readyState === WebSocket.OPEN) {
      console.log('[WebSocket] Already connected');
      return;
    }

    try {
      // 构建 WebSocket URL
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = url.startsWith('ws') ? url : `${protocol}//${window.location.host}${url}`;

      console.log('[WebSocket] Connecting to:', wsUrl);
      ws.value = new WebSocket(wsUrl);

      ws.value.onopen = () => {
        console.log('[WebSocket] Connected');
        isConnected.value = true;
        reconnectAttempts.value = 0;

        // 触发连接成功事件
        triggerEvent('connected');
      };

      ws.value.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          console.log('[WebSocket] Message received:', data);

          // 触发对应类型的处理器
          if (data.type && messageHandlers.has(data.type)) {
            const handlers = messageHandlers.get(data.type);
            handlers.forEach(handler => handler(data));
          }

          // 触发通用消息处理器
          triggerEvent('message', data);
        } catch (error) {
          console.error('[WebSocket] Failed to parse message:', error);
        }
      };

      ws.value.onerror = (error) => {
        console.error('[WebSocket] Error:', error);
        triggerEvent('error', error);
      };

      ws.value.onclose = (event) => {
        console.log('[WebSocket] Disconnected:', event.code, event.reason);
        isConnected.value = false;
        triggerEvent('disconnected');

        // 尝试重连
        if (reconnectAttempts.value < maxReconnectAttempts) {
          const delay = Math.min(1000 * Math.pow(2, reconnectAttempts.value), 30000);
          console.log(`[WebSocket] Reconnecting in ${delay}ms (attempt ${reconnectAttempts.value + 1}/${maxReconnectAttempts})`);

          reconnectInterval.value = setTimeout(() => {
            reconnectAttempts.value++;
            connect();
          }, delay);
        } else {
          console.error('[WebSocket] Max reconnect attempts reached');
        }
      };
    } catch (error) {
      console.error('[WebSocket] Connection failed:', error);
      triggerEvent('error', error);
    }
  };

  const disconnect = () => {
    if (reconnectInterval.value) {
      clearTimeout(reconnectInterval.value);
      reconnectInterval.value = null;
    }

    if (ws.value) {
      console.log('[WebSocket] Disconnecting...');
      ws.value.close();
      ws.value = null;
    }

    isConnected.value = false;
  };

  const send = (data) => {
    if (ws.value && ws.value.readyState === WebSocket.OPEN) {
      const message = typeof data === 'string' ? data : JSON.stringify(data);
      ws.value.send(message);
      console.log('[WebSocket] Message sent:', message);
      return true;
    } else {
      console.warn('[WebSocket] Cannot send message - not connected');
      return false;
    }
  };

  const on = (type, handler) => {
    if (!messageHandlers.has(type)) {
      messageHandlers.set(type, []);
    }
    messageHandlers.get(type).push(handler);

    // 返回取消订阅函数
    return () => {
      const handlers = messageHandlers.get(type);
      if (handlers) {
        const index = handlers.indexOf(handler);
        if (index > -1) {
          handlers.splice(index, 1);
        }
      }
    };
  };

  const off = (type, handler) => {
    const handlers = messageHandlers.get(type);
    if (handlers && handler) {
      const index = handlers.indexOf(handler);
      if (index > -1) {
        handlers.splice(index, 1);
      }
    } else if (type) {
      messageHandlers.delete(type);
    }
  };

  const triggerEvent = (type, data) => {
    if (messageHandlers.has(type)) {
      const handlers = messageHandlers.get(type);
      handlers.forEach(handler => handler(data));
    }
  };

  onMounted(() => {
    // 自动连接可以在组件中手动调用 connect()
    // connect();
  });

  onUnmounted(() => {
    disconnect();
  });

  return {
    isConnected,
    reconnectAttempts,
    connect,
    disconnect,
    send,
    on,
    off
  };
}
