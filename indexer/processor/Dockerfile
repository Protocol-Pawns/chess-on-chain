FROM node:22-slim AS base
RUN corepack enable && corepack prepare yarn@4.16.0 --activate
WORKDIR /app

COPY package.json yarn.lock .yarnrc.yml ./
COPY indexer/processor/package.json indexer/processor/package.json

FROM base AS deps
RUN yarn workspaces focus processor

FROM base
COPY --from=deps /app/node_modules ./node_modules
COPY indexer/processor/src ./indexer/processor/src

WORKDIR /app/indexer/processor
CMD ["npx", "tsx", "src/main.ts"]
