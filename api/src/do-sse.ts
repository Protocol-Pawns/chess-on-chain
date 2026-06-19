interface SSEConnection {
  accountIds: Set<string>;
  enqueue: (data: string) => void;
  close: () => void;
}

export class SSEHub implements DurableObject {
  private connections: SSEConnection[] = [];
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private notifyRateLimits: Map<string, number> = new Map();

  private ensureHeartbeat() {
    if (this.heartbeatTimer) return;
    this.heartbeatTimer = setInterval(() => {
      this.sendAll('event: heartbeat\ndata: {}\n\n');
    }, 5000);
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

  private fanOut(
    eventType: string,
    triggerBlockHeight: number,
    triggerBlockTimestamp: number,
    eventData: Record<string, unknown>,
    targets: string[]
  ): number {
    if (!targets || targets.length === 0) return 0;
    const payload = `event: ${eventType}\ndata: ${JSON.stringify({
      trigger_block_height: triggerBlockHeight,
      trigger_block_timestamp: triggerBlockTimestamp,
      event_data: eventData
    })}\n\n`;
    let delivered = 0;
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
    return delivered;
  }

  private sweepDead() {
    this.connections = this.connections.filter(c => {
      try {
        c.enqueue('');
        return true;
      } catch {
        return false;
      }
    });
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
        delivered += this.fanOut(
          event.event_type,
          event.trigger_block_height,
          event.trigger_block_timestamp,
          event.event_data,
          event.target_accounts
        );
      }

      this.sweepDead();
      return Response.json({ ok: true, delivered });
    }

    if (url.pathname === '/notify-move' && request.method === 'POST') {
      const body = (await request.json()) as {
        notifier: string;
        target: string;
        event_data: Record<string, unknown>;
      };

      if (!body.notifier || !body.target) {
        return Response.json(
          { ok: false, error: 'Missing notifier or target' },
          { status: 400 }
        );
      }

      const now = Date.now();
      const last = this.notifyRateLimits.get(body.notifier) ?? 0;
      if (now - last < 1000) {
        return Response.json({ ok: false, rate_limited: true });
      }
      this.notifyRateLimits.set(body.notifier, now);
      if (this.notifyRateLimits.size > 1000) {
        for (const [key, ts] of this.notifyRateLimits) {
          if (now - ts > 10000) this.notifyRateLimits.delete(key);
        }
      }

      const delivered = this.fanOut('play_move_tx', 0, now, body.event_data, [
        body.target
      ]);

      this.sweepDead();
      return Response.json({ ok: true, delivered });
    }

    return new Response('Not found', { status: 404 });
  }

  async alarm() {}
}
