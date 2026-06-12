const API_URL =
  import.meta.env.VITE_API_URL || 'https://api.protocol-pawns.com';

export interface SSEEventData {
  trigger_block_height: number;
  trigger_block_timestamp: number;
  event_data: Record<string, unknown>;
}

export type SSEEventHandler = (event: SSEEventData) => void;
export type ReconnectHandler = () => void;

let es: EventSource | null = null;
let accountId: string | null = null;
let reconnectDelay = 1000;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
const handlers: Map<string, Set<SSEEventHandler>> = new Map();
const reconnectHandlers: Set<ReconnectHandler> = new Set();
let blockHeightWatermark = 0;
let globalHandler: ((eventType: string, data: SSEEventData) => void) | null =
  null;

let lastHeartbeat = 0;
let heartbeatTimer: ReturnType<typeof setInterval> | null = null;
const HEARTBEAT_TIMEOUT_MS = 10000;

function startHeartbeatMonitor() {
  stopHeartbeatMonitor();
  lastHeartbeat = Date.now();
  heartbeatTimer = setInterval(() => {
    if (Date.now() - lastHeartbeat > HEARTBEAT_TIMEOUT_MS) {
      console.warn('[SSE] heartbeat timeout, reconnecting');
      reconnect();
    }
  }, 3000);
}

function stopHeartbeatMonitor() {
  if (heartbeatTimer) {
    clearInterval(heartbeatTimer);
    heartbeatTimer = null;
  }
}

function reconnect() {
  disconnect();
  reconnectDelay = 1000;
  connect();
  for (const h of reconnectHandlers) h();
}

function connect() {
  if (!accountId) return;

  const url = `${API_URL}/events?account=${encodeURIComponent(accountId)}`;
  es = new EventSource(url);

  es.onopen = () => {
    reconnectDelay = 1000;
    startHeartbeatMonitor();
  };

  es.onerror = () => {
    stopHeartbeatMonitor();
    disconnect();
    reconnectTimer = setTimeout(() => {
      reconnectDelay = Math.min(reconnectDelay * 2, 30000);
      connect();
      for (const h of reconnectHandlers) h();
    }, reconnectDelay);
  };

  es.addEventListener('heartbeat', () => {
    lastHeartbeat = Date.now();
  });

  es.onmessage = e => {
    if (e.data === '' || e.data === '{}') return;
  };

  for (const eventType of handlers.keys()) {
    listenForType(eventType);
  }
}

function listenForType(eventType: string) {
  if (!es) return;
  es.addEventListener(eventType, (e: MessageEvent) => {
    try {
      const data: SSEEventData = JSON.parse(e.data);
      if (data.trigger_block_height <= blockHeightWatermark) return;
      const typeHandlers = handlers.get(eventType);
      if (typeHandlers) {
        for (const h of typeHandlers) h(data);
      }
      if (globalHandler) globalHandler(eventType, data);
    } catch {
      /* */
    }
  });
}

export function connectSSE(account: string) {
  if (accountId === account && es) return;
  disconnect();
  accountId = account;
  reconnectDelay = 1000;
  connect();
}

export function disconnectSSE() {
  accountId = null;
  blockHeightWatermark = 0;
  stopHeartbeatMonitor();
  disconnect();
}

function disconnect() {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  if (es) {
    es.close();
    es = null;
  }
}

export function subscribe(
  eventType: string,
  handler: SSEEventHandler
): () => void {
  let set = handlers.get(eventType);
  if (!set) {
    set = new Set();
    handlers.set(eventType, set);
    if (es) listenForType(eventType);
  }
  set.add(handler);
  return () => {
    set!.delete(handler);
    if (set!.size === 0) handlers.delete(eventType);
  };
}

export function onReconnect(handler: ReconnectHandler): () => void {
  reconnectHandlers.add(handler);
  return () => {
    reconnectHandlers.delete(handler);
  };
}

export function setGlobalHandler(
  handler: ((eventType: string, data: SSEEventData) => void) | null
) {
  globalHandler = handler;
}

export function setBlockHeightWatermark(height: number) {
  if (height > blockHeightWatermark) blockHeightWatermark = height;
}

export function updateWatermark(height: number) {
  if (height > blockHeightWatermark) blockHeightWatermark = height;
}
