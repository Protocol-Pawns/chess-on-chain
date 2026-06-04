import { createRoute } from '@hono/zod-openapi';
import { z } from 'zod';

import {
  AccountSchema,
  AccountStatsSchema,
  ChallengeSchema,
  EloLeaderboardPageSchema,
  GameIdSchema,
  GameMoveSchema,
  GameOverviewSchema,
  GameSchema,
  GlobalStatsSchema,
  InfoSchema,
  PaginatedGamesSchema,
  PaginatedLeaderboardSchema,
  PushSubscriptionSchema,
  VapidPublicKeySchema
} from './events';

const NotFoundSchema = z.object({ error: z.literal('Not found') });

export const getInfoRoute = createRoute({
  method: 'get',
  path: '/info',
  responses: {
    200: {
      content: { 'application/json': { schema: InfoSchema } },
      description: 'Returns the last indexed block height'
    }
  }
});

export const getGlobalStatsRoute = createRoute({
  method: 'get',
  path: '/stats',
  responses: {
    200: {
      content: { 'application/json': { schema: GlobalStatsSchema } },
      description: 'Returns global platform statistics'
    }
  }
});

export const getGameRoute = createRoute({
  method: 'get',
  path: '/game/{game_id}',
  request: {
    params: z.object({ game_id: z.string() })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: GameSchema } },
      description: 'Returns a game by ID'
    },
    404: {
      content: { 'application/json': { schema: NotFoundSchema } },
      description: 'Game not found'
    }
  }
});

export const getGameMovesRoute = createRoute({
  method: 'get',
  path: '/game/{game_id}/moves',
  request: {
    params: z.object({ game_id: z.string() })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: GameMoveSchema.array() } },
      description: 'Returns move history for a game'
    }
  }
});

export const queryGamesRoute = createRoute({
  method: 'post',
  path: '/query',
  request: {
    body: {
      content: {
        'application/json': {
          schema: z.object({
            gameIds: GameIdSchema.array(),
            includeMoves: z.boolean().optional()
          })
        }
      }
    }
  },
  responses: {
    200: {
      content: { 'application/json': { schema: GameOverviewSchema.array() } },
      description: 'Returns games matching the given IDs'
    }
  }
});

export const getGamesRoute = createRoute({
  method: 'get',
  path: '/games',
  request: {
    query: z.object({
      status: z.enum(['active', 'finished']).optional().default('active'),
      cursor: z.string().optional(),
      limit: z.string().optional(),
      include_moves: z.string().optional()
    })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: PaginatedGamesSchema } },
      description: 'Returns paginated games filtered by status'
    }
  }
});

export const getActiveGameRoute = createRoute({
  method: 'get',
  path: '/account/{account_id}/active-game',
  request: {
    params: z.object({ account_id: z.string() })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: GameSchema } },
      description: 'Returns the active game for an account'
    },
    404: {
      content: { 'application/json': { schema: NotFoundSchema } },
      description: 'No active game found'
    }
  }
});

export const getAccountRoute = createRoute({
  method: 'get',
  path: '/account/{account_id}',
  request: {
    params: z.object({ account_id: z.string() })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: AccountSchema } },
      description: 'Returns account data with finished game IDs'
    }
  }
});

export const getAccountStatsRoute = createRoute({
  method: 'get',
  path: '/account/{account_id}/stats',
  request: {
    params: z.object({ account_id: z.string() })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: AccountStatsSchema } },
      description: 'Returns win/loss/draw statistics for an account'
    }
  }
});

export const getChallengesRoute = createRoute({
  method: 'get',
  path: '/account/{account_id}/challenges',
  request: {
    params: z.object({ account_id: z.string() })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: ChallengeSchema.array() } },
      description: 'Returns challenges for an account'
    }
  }
});

export const getLeaderboardRoute = createRoute({
  method: 'get',
  path: '/leaderboard',
  request: {
    query: z.object({
      cursor: z.string().optional(),
      limit: z.string().optional()
    })
  },
  responses: {
    200: {
      content: {
        'application/json': { schema: PaginatedLeaderboardSchema }
      },
      description: 'Returns top players ranked by wins'
    }
  }
});

export const getLeaderboardEloRoute = createRoute({
  method: 'get',
  path: '/leaderboard/elo',
  request: {
    query: z.object({
      page: z.string().optional().default('1'),
      per_page: z.string().optional().default('25')
    })
  },
  responses: {
    200: {
      content: {
        'application/json': { schema: EloLeaderboardPageSchema }
      },
      description: 'Returns ELO-ranked leaderboard with pagination'
    }
  }
});

export const getVapidPublicKeyRoute = createRoute({
  method: 'get',
  path: '/vapid-public-key',
  responses: {
    200: {
      content: { 'application/json': { schema: VapidPublicKeySchema } },
      description: 'Returns the VAPID public key for push subscriptions'
    }
  }
});

export const subscribePushRoute = createRoute({
  method: 'post',
  path: '/account/{account_id}/push-subscription',
  request: {
    params: z.object({ account_id: z.string() }),
    body: {
      content: {
        'application/json': {
          schema: PushSubscriptionSchema
        }
      }
    }
  },
  responses: {
    200: {
      content: {
        'application/json': { schema: z.object({ ok: z.boolean() }) }
      },
      description: 'Push subscription registered'
    }
  }
});

export const unsubscribePushRoute = createRoute({
  method: 'delete',
  path: '/account/{account_id}/push-subscription',
  request: {
    params: z.object({ account_id: z.string() }),
    body: {
      content: {
        'application/json': {
          schema: z.object({ endpoint: z.string() })
        }
      }
    }
  },
  responses: {
    200: {
      content: {
        'application/json': { schema: z.object({ ok: z.boolean() }) }
      },
      description: 'Push subscription removed'
    }
  }
});
