import { createRoute } from '@hono/zod-openapi';
import { z } from 'zod';

import {
  AccountSchema,
  GameIdSchema,
  GameOverviewSchema,
  GameSchema,
  InfoSchema
} from './events';

export const getInfoRoute = createRoute({
  method: 'get',
  path: '/',
  responses: {
    200: {
      content: { 'application/json': { schema: InfoSchema } },
      description: 'Returns the last indexed block height'
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
    404: { description: 'Game not found' }
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

export const getRecentNewGamesRoute = createRoute({
  method: 'get',
  path: '/recent/new',
  request: {
    query: z.object({
      limit: z.string().optional(),
      include_moves: z.string().optional()
    })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: GameOverviewSchema.array() } },
      description: 'Returns recent new games'
    }
  }
});

export const getRecentFinishedGamesRoute = createRoute({
  method: 'get',
  path: '/recent/finished',
  request: {
    query: z.object({
      limit: z.string().optional(),
      include_moves: z.string().optional()
    })
  },
  responses: {
    200: {
      content: { 'application/json': { schema: GameOverviewSchema.array() } },
      description: 'Returns recent finished games'
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
