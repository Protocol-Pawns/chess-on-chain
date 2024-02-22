import { zValidator } from '@hono/zod-validator';
import { Hono } from 'hono';
import { bearerAuth } from 'hono/bearer-auth';
import { P, match } from 'ts-pattern';

import { zodBatchEvent } from './events';
import type { Env } from './global';

export const batch = new Hono<{ Bindings: Env }>();
batch
  .use('*', async (c, next) => {
    const auth = bearerAuth({ token: c.env.INDEXER_SECRET });
    await auth(c, next);
  })
  .post(
    '',
    zValidator('json', zodBatchEvent, result => {
      if (result.success) return;
      console.info(result.error.errors);
    }),
    async c => {
      const batchEvent = c.req.valid('json');

      console.info(
        `[${new Date().toLocaleString()}] block_height ${batchEvent.block_height}`
      );

      const infoAddr = c.env.INFO.idFromName('');
      const infoStub = c.env.INFO.get(infoAddr);
      const gameAddr = c.env.GAMES.idFromName('');
      const gameStub = c.env.GAMES.get(gameAddr);

      const locks: Record<string, Promise<void> | undefined> = {};

      await infoStub.fetch(`${new URL(c.req.url).origin}/last_block_height`, {
        method: 'POST',
        body: String(batchEvent.block_height)
      });

      if (batchEvent.events.length > 0) {
        for (const event of batchEvent.events) {
          try {
            const gameId = JSON.stringify(event.data.game_id);
            if (locks[gameId]) {
              await locks[gameId];
            }
            locks[gameId] = new Promise<void>((resolve, reject) => {
              try {
                match(event)
                  .with(
                    { event: 'create_game', data: P.select() },
                    async createGame => {
                      console.info('create_game', createGame);

                      await awaitResponse(
                        gameStub.fetch(
                          `${new URL(c.req.url).origin}/${encodeURI(JSON.stringify(createGame.game_id))}/create_game`,
                          {
                            method: 'POST',
                            body: JSON.stringify(createGame)
                          }
                        ),
                        reject
                      );
                      resolve();
                    }
                  )
                  .with(
                    { event: 'play_move', data: P.select() },
                    async playMove => {
                      console.info('play_move', playMove);

                      await awaitResponse(
                        gameStub.fetch(
                          `${new URL(c.req.url).origin}/${encodeURI(JSON.stringify(playMove.game_id))}/play_move`,
                          {
                            method: 'POST',
                            body: JSON.stringify(playMove)
                          }
                        ),
                        reject
                      );
                      resolve();
                    }
                  )
                  .with(
                    { event: 'resign_game', data: P.select() },
                    async resignGame => {
                      console.info('resign_game', resignGame);

                      await awaitResponse(
                        gameStub.fetch(
                          `${new URL(c.req.url).origin}/${encodeURI(JSON.stringify(resignGame.game_id))}/resign_game`,
                          {
                            method: 'POST',
                            body: JSON.stringify(resignGame)
                          }
                        ),
                        reject
                      );
                      resolve();
                    }
                  )
                  .with(
                    { event: 'cancel_game', data: P.select() },
                    async cancelGame => {
                      console.info('cancel_game', cancelGame);

                      await awaitResponse(
                        gameStub.fetch(
                          `${new URL(c.req.url).origin}/${encodeURI(JSON.stringify(cancelGame.game_id))}/cancel_game`,
                          {
                            method: 'POST',
                            body: JSON.stringify(cancelGame)
                          }
                        ),
                        reject
                      );
                      resolve();
                    }
                  )
                  .exhaustive();
              } catch (err) {
                reject();
              }
            });
          } catch (err) {
            if (err instanceof Response) {
              return err;
            } else {
              console.error(`Unexpected error: ${err}`);
            }
          }
        }
      }

      return new Response(null, { status: 204 });
    }
  );

async function awaitResponse(
  promise: Promise<Response>,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  reject: (reason?: any) => void
) {
  try {
    const res = await promise;
    if (!res.ok) {
      rejectWithError(res, reject);
      return;
    }
  } catch (err) {
    rejectWithError(new Response('', { status: 500 }), reject);
  }
}

async function rejectWithError(
  res: Response,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  reject: (reason?: any) => void
) {
  console.error(
    `Response from ${res.url} returned error: [${
      res.status
    }] ${await res.text()}`
  );
  reject(new Response(null, { status: 500 }));
}
