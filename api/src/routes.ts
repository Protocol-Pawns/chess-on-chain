import { createRoute } from '@hono/zod-openapi';
import { z } from 'zod';

import {
  AccountSchema,
  AccountSearchResultSchema,
  AccountStatsSchema,
  BetLeaderboardEntrySchema,
  BetSchema,
  BetStatsSchema,
  ChallengeSchema,
  GameIdSchema,
  GameMoveSchema,
  GameOverviewSchema,
  GameSchema,
  GlobalStatsSchema,
  InfoSchema,
  PaginatedBetsSchema,
  PaginatedChallengesSchema,
  PaginatedGamesSchema,
  PushSubscriptionSchema,
  RankingPageSchema,
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
      include_moves: z.string().optional(),
      page: z.string().optional(),
      exclude_ai: z
        .enum(['true', '1'])
        .optional()
        .transform(v => v === 'true' || v === '1')
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

export const batchAccountStatsRoute = createRoute({
  method: 'post',
  path: '/account/stats/batch',
  request: {
    body: {
      content: {
        'application/json': {
          schema: z.object({ account_ids: z.string().array() })
        }
      }
    }
  },
  responses: {
    200: {
      content: { 'application/json': { schema: AccountStatsSchema.array() } },
      description: 'Returns win/loss/draw statistics for multiple accounts'
    }
  }
});

export const searchAccountsRoute = createRoute({
  method: 'post',
  path: '/account/query',
  request: {
    body: {
      content: {
        'application/json': {
          schema: z.object({ query: z.string().min(1).max(64) })
        }
      }
    }
  },
  responses: {
    200: {
      content: {
        'application/json': { schema: AccountSearchResultSchema.array() }
      },
      description:
        'Returns accounts matching the query prefix with ELO and win/loss/draw stats'
    }
  }
});

export const getChallengesRoute = createRoute({
  method: 'get',
  path: '/account/{account_id}/challenges',
  request: {
    params: z.object({ account_id: z.string() }),
    query: z.object({
      page: z.string().optional(),
      per_page: z.string().optional(),
      exclude_rejected: z
        .enum(['true', '1'])
        .optional()
        .transform(v => v === 'true' || v === '1')
    })
  },
  responses: {
    200: {
      content: {
        'application/json': {
          schema: z.union([ChallengeSchema.array(), PaginatedChallengesSchema])
        }
      },
      description: 'Returns challenges for an account'
    }
  }
});

export const getLeaderboardEloRoute = createRoute({
  method: 'get',
  path: '/leaderboard/elo',
  request: {
    query: z.object({
      page: z.string().optional().default('1'),
      per_page: z.string().optional().default('25'),
      dir: z.enum(['desc', 'asc']).optional().default('desc')
    })
  },
  responses: {
    200: {
      content: {
        'application/json': { schema: RankingPageSchema }
      },
      description:
        'Returns ELO-ranked leaderboard with pagination, enriched with PPP and stats'
    }
  }
});

export const getLeaderboardPppRoute = createRoute({
  method: 'get',
  path: '/leaderboard/ppp',
  request: {
    query: z.object({
      page: z.string().optional().default('1'),
      per_page: z.string().optional().default('25')
    })
  },
  responses: {
    200: {
      content: {
        'application/json': { schema: RankingPageSchema }
      },
      description:
        'Returns PPP-ranked leaderboard (max 100 from FastNear), enriched with ELO and stats'
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

export const getBetsRoute = createRoute({
  method: 'get',
  path: '/account/{account_id}/bets',
  request: {
    params: z.object({ account_id: z.string() }),
    query: z.object({
      status: z.enum(['pending', 'locked', 'resolved']).optional(),
      cursor: z.string().optional(),
      limit: z.string().optional()
    })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: PaginatedBetsSchema } },
      description: 'Returns paginated bets placed by an account'
    }
  }
});

export const getGameBetsRoute = createRoute({
  method: 'get',
  path: '/game/{game_id}/bets',
  request: {
    params: z.object({ game_id: z.string() })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: BetSchema.array() } },
      description: 'Returns all bets for a specific game'
    }
  }
});

export const getBetStatsRoute = createRoute({
  method: 'get',
  path: '/account/{account_id}/bet-stats',
  request: {
    params: z.object({ account_id: z.string() })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: BetStatsSchema } },
      description: 'Returns aggregate betting statistics for an account'
    }
  }
});

export const getBetLeaderboardRoute = createRoute({
  method: 'get',
  path: '/leaderboard/bets',
  request: {
    query: z.object({
      cursor: z.string().optional(),
      limit: z.string().optional()
    })
  },
  responses: {
    200: {
      content: {
        'application/json': {
          schema: z.object({
            items: BetLeaderboardEntrySchema.array(),
            next_cursor: z.string().nullable()
          })
        }
      },
      description: 'Returns top bettors ranked by total winnings'
    }
  }
});

export const getOpenChallengesRoute = createRoute({
  method: 'get',
  path: '/challenges',
  request: {
    query: z.object({
      cursor: z.string().optional(),
      limit: z.string().optional()
    })
  },
  responses: {
    200: {
      content: {
        'application/json': { schema: PaginatedChallengesSchema }
      },
      description: 'Returns paginated open (pending) challenges'
    }
  }
});

export const getGlobalBetsRoute = createRoute({
  method: 'get',
  path: '/bets',
  request: {
    query: z.object({
      status: z.enum(['pending', 'locked', 'resolved']).optional(),
      cursor: z.string().optional(),
      limit: z.string().optional()
    })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: PaginatedBetsSchema } },
      description: 'Returns paginated bets across all accounts'
    }
  }
});
