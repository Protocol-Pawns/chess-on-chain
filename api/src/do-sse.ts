interface SSEConnection {
  accountIds: Set<string>;
  enqueue: (data: string) => void;
  close: () => void;
}

export class SSEHub implements DurableObject {
  private connections: SSEConnection[] = [];
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;

  private ensureHeartbeat() {
    if (this.heartbeatTimer) return;
    this.heartbeatTimer = setInterval(() => {
      this.sendAll(': heartbeat\n\n');
    }, 30000);
  }

  private stopHeartbeat() {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private sendAll(data: string) {
    const alive: SSEConnection[] = [];
    for (const conn of this.connections) {
      try {
        conn.enqueue(data);
        alive.push(conn);
      } catch {
        try {
          conn.close();
        } catch {
          /* */
        }
      }
    }
    this.connections = alive;
    if (this.connections.length === 0) this.stopHeartbeat();
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/subscribe' && request.method === 'GET') {
      const accounts = url.searchParams.getAll('account');
      if (accounts.length === 0) {
        return new Response('Missing account param', { status: 400 });
      }
      const accountIds = new Set(accounts);

      const stream = new ReadableStream({
        start: controller => {
          const enqueue = (data: string) => controller.enqueue(data);
          const close = () => {
            try {
              controller.close();
            } catch {
              /* */
            }
          };
          const conn: SSEConnection = { accountIds, enqueue, close };
          this.connections.push(conn);
          this.ensureHeartbeat();

          enqueue(': connected\n\n');

          request.signal.addEventListener('abort', () => {
            this.connections = this.connections.filter(c => c !== conn);
            close();
            if (this.connections.length === 0) this.stopHeartbeat();
          });
        }
      });

      return new Response(stream, {
        headers: {
          'Content-Type': 'text/event-stream',
          'Cache-Control': 'no-cache, no-transform',
          Connection: 'keep-alive',
          'X-Accel-Buffering': 'no'
        }
      });
    }

    if (url.pathname === '/publish' && request.method === 'POST') {
      const body = (await request.json()) as {
        events: Array<{
          event_type: string;
          trigger_block_height: number;
          trigger_block_timestamp: number;
          event_data: Record<string, unknown>;
          target_accounts: string[];
        }>;
      };

      let delivered = 0;
      for (const event of body.events) {
        const targets = event.target_accounts;
        if (!targets || targets.length === 0) continue;

        const payload = `event: ${event.event_type}\ndata: ${JSON.stringify({
          trigger_block_height: event.trigger_block_height,
          trigger_block_timestamp: event.trigger_block_timestamp,
          event_data: event.event_data
        })}\n\n`;

        for (const conn of this.connections) {
          const matches = targets.some(t => conn.accountIds.has(t));
          if (!matches) continue;
          try {
            conn.enqueue(payload);
            delivered++;
          } catch {
            try {
              conn.close();
            } catch {
              /* */
            }
          }
        }
      }

      this.connections = this.connections.filter(c => {
        try {
          c.enqueue('');
          return true;
        } catch {
          return false;
        }
      });
      if (this.connections.length === 0) this.stopHeartbeat();

      return Response.json({ ok: true, delivered });
    }

    return new Response('Not found', { status: 404 });
  }

  async alarm() {}
}
