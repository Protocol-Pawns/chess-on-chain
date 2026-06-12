<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { Chess } from 'chess.js';
  import {
    api,
    type GameMove,
    type Bet,
    type GameOverview
  } from '$lib/api/client';
  import { contract, getTxLogs } from '$lib/near/connector';
  import { accountStore } from '$lib/near/account';
  import { colorFromFEN } from '$lib/chess/board';
  import { showToast } from '$lib/toast';
  import { truncateAddr } from '$lib/format';
  import { loadGameFromContract, boardToFen, parseGamePath } from '$lib/game';
  import type { GameId, ContractGameData } from '$lib/game';
  import type { SSEEventData } from '$lib/sse';
  import {
    subscribe,
    updateWatermark,
    onReconnect,
    connectSSE
  } from '$lib/sse';
  import { get } from 'svelte/store';
  import Board from '$lib/components/Board.svelte';
  import MoveHistory from '$lib/components/MoveHistory.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';

  import dayjs from 'dayjs';

  function parseMoveNotation(
    notation: string,
    color?: string
  ): { from: string; to: string } | null {
    if (/^O-O-O$/i.test(notation) || /^0-0-0$/.test(notation)) {
      const rank = color === 'Black' ? '8' : '1';
      return { from: `e${rank}`, to: `c${rank}` };
    }
    if (/^O-O$/i.test(notation) || /^0-0$/.test(notation)) {
      const rank = color === 'Black' ? '8' : '1';
      return { from: `e${rank}`, to: `g${rank}` };
    }
    const parts = notation.split(' to ');
    if (parts.length >= 2) {
      const from = parts[0].trim();
      const to = parts[1].trim().split(/\s/)[0];
      if (/^[a-h][1-8]$/.test(from) && /^[a-h][1-8]$/.test(to)) {
        return { from, to };
      }
    }
    return null;
  }

  let game = $state<GameOverview | null>(null);
  let moves = $state<GameMove[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let waiting = $state(false);
  let submitting = $state(false);
  let indexed = $state(false);
  let gameBets = $state<Bet[]>([]);
  let showResignModal = $state(false);
  let showCancelModal = $state(false);
  let showPublicCancelModal = $state(false);
  let pendingLastMove = $state<{ from: string; to: string } | null>(null);
  let viewingMoveIndex = $state<number | null>(null);
  let appliedMoveSigs = $state<Set<string>>(new Set());

  function moveSig(color: string, mv: string): string {
    return `${color}:${mv}`;
  }
  const STARTING_FEN =
    'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

  const gameId: GameId = parseGamePath(page.params.id ?? '');
  const gameIdStr = JSON.stringify(gameId);

  let lastMove = $derived(
    pendingLastMove ??
      (moves.length > 0
        ? parseMoveNotation(
            moves[moves.length - 1].move_notation,
            moves[moves.length - 1].color
          )
        : null)
  );

  let isViewingCurrent = $derived(viewingMoveIndex === null);

  let displayFen = $derived.by(() => {
    if (isViewingCurrent) return game?.fen ?? undefined;
    if (viewingMoveIndex === -1) return STARTING_FEN;
    if (
      viewingMoveIndex != null &&
      viewingMoveIndex >= 0 &&
      viewingMoveIndex < moves.length
    )
      return moves[viewingMoveIndex].fen;
    return game?.fen ?? undefined;
  });

  let displayBoard = $derived.by(() => {
    if (isViewingCurrent) return game?.board;
    return undefined;
  });

  let displayLastMove = $derived.by(() => {
    if (isViewingCurrent) return lastMove;
    if (viewingMoveIndex === -1) return null;
    if (
      viewingMoveIndex != null &&
      viewingMoveIndex >= 0 &&
      viewingMoveIndex < moves.length
    ) {
      return parseMoveNotation(
        moves[viewingMoveIndex].move_notation,
        moves[viewingMoveIndex].color
      );
    }
    return null;
  });

  function selectMove(index: number | null) {
    viewingMoveIndex = index;
  }

  let isMyTurn = $derived.by(() => {
    if (!game || game.status !== 'in_progress' || !$accountStore) return false;
    const turn = game.fen ? colorFromFEN(game.fen) : 'White';
    const myColor =
      game.white.value === $accountStore
        ? 'White'
        : game.black?.value === $accountStore
          ? 'Black'
          : null;
    return turn === myColor;
  });

  let contractTurnColor = $state<string | null>(null);

  let currentTurn = $derived(
    game?.status === 'in_progress'
      ? game.fen
        ? colorFromFEN(game.fen)
        : contractTurnColor
      : null
  );

  let flipped = $derived(
    !!(game && $accountStore && game.black?.value === $accountStore)
  );

  let canResign = $derived(
    game?.status === 'in_progress' &&
      $accountStore &&
      (game.white.value === $accountStore ||
        game.black?.value === $accountStore)
  );
  let canCancel = $derived(
    game?.status === 'waiting' &&
      $accountStore &&
      (game.white.value === $accountStore ||
        game.black?.value === $accountStore)
  );
  let isSpectating = $derived(
    $accountStore &&
      game &&
      game.white.value !== $accountStore &&
      game.black?.value !== $accountStore
  );
  let canPublicCancel = $derived(
    game?.status === 'in_progress' &&
      $accountStore &&
      isSpectating &&
      game.created_at &&
      dayjs().diff(dayjs(game.created_at), 'day') >= 14
  );

  async function loadFromContract(): Promise<ContractGameData> {
    return loadGameFromContract(gameId);
  }

  let localMoveCount = $state(0);
  async function load() {
    try {
      const [g, m] = await Promise.all([
        api.game(gameIdStr),
        api.gameMoves(gameIdStr)
      ]);
      error = null;
      waiting = false;
      indexed = true;
      if (game && game.status !== 'in_progress' && game.status !== g.status) {
        return;
      }
      if (m.length < localMoveCount && game?.status === 'in_progress') {
        return;
      }
      const localAhead = localMoveCount > m.length;
      if (localAhead) {
        const { board: localBoard, fen: localFen } = game ?? {};
        game = {
          ...g,
          board: localBoard ?? g.board,
          fen: localFen ?? g.fen
        };
      } else {
        game = g;
      }
      moves = m;
      localMoveCount = Math.max(localMoveCount, m.length);
      if (!localAhead && m.length > 0) pendingLastMove = null;
      contractTurnColor = null;
      const sigs = new Set<string>();
      for (const mv of m) sigs.add(moveSig(mv.color, mv.move_notation));
      appliedMoveSigs = sigs;
      if (!localAhead) {
        gameBets = await api.gameBets(gameIdStr).catch(() => []);
      }
      if (g.status === 'finished' || g.status === 'cancelled') {
        stopPolling();
      }
    } catch (e) {
      if (game?.status !== 'in_progress' && game?.status !== undefined) return;

      console.warn('[game] API load failed, falling back to contract:', e);
      try {
        const contractGame = await loadFromContract();
        game = { ...contractGame } as GameOverview;
        contractTurnColor = contractGame.turn_color;
        moves = [];
        localMoveCount = 0;
        gameBets = [];
        waiting = false;
        indexed = false;
      } catch (e2) {
        console.warn('[game] contract fallback also failed, will retry:', e2);
        if (!game) waiting = true;
      }
    } finally {
      loading = false;
    }
  }

  function parseTxLogs(logs: string[]) {
    const parsedMoves: {
      from: string;
      to: string;
      color: string;
      mv: string;
    }[] = [];
    let outcome: Record<string, unknown> | null = null;
    let board: string[] | null = null;
    let resigner: string | null = null;
    let cancelled = false;
    for (const log of logs) {
      try {
        const json = log.startsWith('EVENT_JSON:') ? log.slice(11) : log;
        const event = JSON.parse(json);
        if (event.standard !== 'chess-game') continue;
        if (event.event === 'play_move') {
          const parts: string[] = event.data.mv.split(' to ');
          if (parts.length === 2) {
            parsedMoves.push({
              from: parts[0],
              to: parts[1],
              color: event.data.color,
              mv: event.data.mv
            });
          }
          if (event.data.board) {
            board = event.data.board as string[];
          }
          if (event.data.outcome) {
            outcome = event.data.outcome as Record<string, unknown>;
          }
        } else if (event.event === 'resign_game') {
          if (event.data.outcome) {
            outcome = event.data.outcome as Record<string, unknown>;
          }
          if (event.data.resigner) {
            resigner = event.data.resigner as string;
          }
        } else if (event.event === 'cancel_game') {
          cancelled = true;
        }
      } catch {
        // skip
      }
    }
    const moves = parsedMoves.map(m => ({ from: m.from, to: m.to }));
    return { moves, outcome, board, resigner, cancelled, parsedMoves };
  }

  function handleMove(from: string, to: string) {
    if (!game || submitting || !indexed) return;
    submitting = true;
    const isAiGame = game.white.type === 'Ai' || game.black?.type === 'Ai';
    contract
      .playMove($state.snapshot(game.game_id), from + to)
      .then(async txResult => {
        submitting = false;
        const tx = txResult as {
          receipts_outcome?: { outcome: { logs: string[] } }[];
          transaction?: { hash?: string };
          transaction_outcome?: { id?: string };
        };
        const txLogs: string[] = [];
        if (tx.receipts_outcome) {
          for (const r of tx.receipts_outcome) {
            txLogs.push(...(r.outcome?.logs ?? []));
          }
        }
        let parsed = parseTxLogs(txLogs);

        if (game) {
          for (const m of parsed.parsedMoves) {
            appliedMoveSigs.add(moveSig(m.color, m.mv));
          }
          localMoveCount += parsed.parsedMoves.length;
          if (parsed.outcome && parsed.board) {
            game = {
              ...game,
              board: parsed.board,
              fen: undefined,
              status: 'finished' as const,
              outcome: parsed.outcome as GameOverview['outcome']
            };
            pendingLastMove = null;
            contractTurnColor = null;
          } else {
            const aiMove =
              isAiGame && parsed.moves.length > 1
                ? parsed.moves[parsed.moves.length - 1]
                : null;
            pendingLastMove = aiMove ?? { from, to };
            if (game.fen) {
              try {
                const c = new Chess(game.fen);
                c.move({ from, to, promotion: 'q' });
                if (aiMove) {
                  c.move({
                    from: aiMove.from,
                    to: aiMove.to,
                    promotion: 'q'
                  });
                }
                game = { ...game, fen: c.fen() };
              } catch {
                if (parsed.board) {
                  game = { ...game, board: parsed.board, fen: undefined };
                }
              }
            } else if (parsed.board) {
              game = { ...game, board: parsed.board, fen: undefined };
            }
          }
        }
      })
      .catch(() => {
        submitting = false;
      });
  }

  function handleResign() {
    showResignModal = true;
  }

  function confirmResign() {
    if (!game) return;
    showResignModal = false;
    submitting = true;
    showToast('info', 'Resigning...');
    contract
      .resign($state.snapshot(game.game_id))
      .then(async txResult => {
        const tx = txResult as {
          receipts_outcome?: { outcome: { logs: string[] } }[];
        };
        const txLogs: string[] = [];
        if (tx.receipts_outcome) {
          for (const r of tx.receipts_outcome) {
            txLogs.push(...(r.outcome?.logs ?? []));
          }
        }
        const parsed = parseTxLogs(txLogs);
        if (parsed.outcome && game) {
          game = {
            ...game,
            status: 'finished' as const,
            outcome: parsed.outcome as GameOverview['outcome'],
            resigner: (parsed.resigner as GameOverview['resigner']) ?? null
          };
          showToast('success', 'Game resigned');
        } else {
          showToast('success', 'Game resigned');
        }
      })
      .catch((err: unknown) => {
        showToast(
          'error',
          'Resign failed',
          err instanceof Error ? err.message : String(err)
        );
      })
      .finally(() => {
        submitting = false;
      });
  }

  function handleCancel() {
    showCancelModal = true;
  }

  function confirmCancel() {
    if (!game) return;
    showCancelModal = false;
    submitting = true;
    showToast('info', 'Cancelling...');
    contract
      .cancel($state.snapshot(game.game_id))
      .then(async txResult => {
        const tx = txResult as {
          receipts_outcome?: { outcome: { logs: string[] } }[];
        };
        const txLogs: string[] = [];
        if (tx.receipts_outcome) {
          for (const r of tx.receipts_outcome) {
            txLogs.push(...(r.outcome?.logs ?? []));
          }
        }
        const parsed = parseTxLogs(txLogs);
        if (parsed.cancelled && game) {
          game = {
            ...game,
            status: 'cancelled' as const
          };
          showToast('success', 'Game cancelled');
        } else {
          showToast('success', 'Game cancelled');
        }
      })
      .catch((err: unknown) => {
        showToast(
          'error',
          'Cancel failed',
          err instanceof Error ? err.message : String(err)
        );
      })
      .finally(() => {
        submitting = false;
      });
  }

  function handlePublicCancel() {
    showPublicCancelModal = true;
  }

  function confirmPublicCancel() {
    if (!game) return;
    showPublicCancelModal = false;
    submitting = true;
    showToast('info', 'Cancelling stale game...');
    contract
      .cancel($state.snapshot(game.game_id))
      .then(async txResult => {
        const tx = txResult as {
          receipts_outcome?: { outcome: { logs: string[] } }[];
        };
        const txLogs: string[] = [];
        if (tx.receipts_outcome) {
          for (const r of tx.receipts_outcome) {
            txLogs.push(...(r.outcome?.logs ?? []));
          }
        }
        const parsed = parseTxLogs(txLogs);
        if (parsed.cancelled && game) {
          game = {
            ...game,
            status: 'cancelled' as const
          };
          showToast('success', 'Stale game cancelled');
        } else {
          showToast('success', 'Game cancelled');
        }
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        if (msg.includes('publicly cancellable')) {
          showToast('error', 'Not yet cancellable', msg);
        } else {
          showToast('error', 'Cancel failed', msg);
        }
      })
      .finally(() => {
        submitting = false;
      });
  }

  function applySSEMove(data: Record<string, unknown>) {
    const board = data.board as string[] | undefined;
    if (!board || !game) return;
    const color = data.color as string;
    const nextTurn: 'White' | 'Black' = color === 'White' ? 'Black' : 'White';
    const fen = boardToFen(board, nextTurn);
    const outcome = data.outcome as { result: string; color?: string } | null;
    const mv = data.mv as string;

    moves = [
      ...moves,
      {
        move_number: moves.length + 1,
        color,
        move_notation: mv,
        fen,
        outcome: outcome ?? null
      }
    ];
    localMoveCount = moves.length;

    game = {
      ...game,
      board,
      fen,
      ...(outcome
        ? {
            status: 'finished' as const,
            outcome,
            ...(data.resigner ? { resigner: data.resigner as string } : {})
          }
        : {})
    };

    pendingLastMove = null;
    contractTurnColor = null;
    gameBets = [];
    api
      .gameBets(gameIdStr)
      .then(b => (gameBets = b))
      .catch(() => {});
  }

  function handleSSEPlayMove(event: SSEEventData) {
    const data = event.event_data;
    const eventGameId =
      typeof data.game_id === 'string'
        ? data.game_id
        : JSON.stringify(data.game_id);
    if (eventGameId !== gameIdStr) return;
    if (game && game.status !== 'in_progress') return;

    const sig = moveSig(data.color as string, data.mv as string);
    if (appliedMoveSigs.has(sig)) return;

    updateWatermark(event.trigger_block_height);
    appliedMoveSigs.add(sig);
    applySSEMove(data);
  }

  function handleSSEResignGame(event: SSEEventData) {
    const data = event.event_data;
    const eventGameId = gameIdFromData(data);
    if (eventGameId !== gameIdStr) return;
    if (!game || game.status !== 'in_progress') return;

    updateWatermark(event.trigger_block_height);
    const outcome = data.outcome as { result: string; color: string };
    game = {
      ...game,
      status: 'finished' as const,
      outcome,
      resigner: data.resigner as string
    };
    showToast('info', 'Opponent resigned!');
  }

  function handleSSECancelGame(event: SSEEventData) {
    const data = event.event_data;
    const eventGameId = gameIdFromData(data);
    if (eventGameId !== gameIdStr) return;
    if (!game || game.status === 'cancelled') return;

    updateWatermark(event.trigger_block_height);
    game = { ...game, status: 'cancelled' as const };
    showToast('info', 'Game was cancelled');
  }

  function handleSSECreateGame(event: SSEEventData) {
    const data = event.event_data;
    const eventGameId = gameIdFromData(data);
    if (eventGameId !== gameIdStr) return;

    updateWatermark(event.trigger_block_height);
    load();
  }

  function gameIdFromData(data: Record<string, unknown>): string {
    return typeof data.game_id === 'string'
      ? data.game_id
      : JSON.stringify(data.game_id);
  }

  let pollInterval: ReturnType<typeof setInterval> | null = null;
  let fastPollRemaining = 0;

  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  onMount(() => {
    const account = get(accountStore);
    if (account) connectSSE(account);

    const unsubAccount = accountStore.subscribe(a => {
      if (a) connectSSE(a);
    });

    load();

    fastPollRemaining = 6;
    pollInterval = setInterval(() => {
      if (game?.status === 'finished' || game?.status === 'cancelled') {
        stopPolling();
        return;
      }
      load();
      if (fastPollRemaining > 0) {
        fastPollRemaining--;
        if (fastPollRemaining === 0 && pollInterval) {
          clearInterval(pollInterval);
          pollInterval = setInterval(load, 5_000);
        }
      }
    }, 2_500);

    const unsubs = [
      subscribe('play_move', handleSSEPlayMove),
      subscribe('resign_game', handleSSEResignGame),
      subscribe('cancel_game', handleSSECancelGame),
      subscribe('create_game', handleSSECreateGame),
      onReconnect(() => load())
    ];

    return () => {
      stopPolling();
      unsubAccount();
      for (const u of unsubs) u();
    };
  });
</script>

{#if loading}
  <div class="flex flex-col gap-4 animate-pulse">
    <div class="card">
      <div class="flex justify-between items-center mb-2">
        <div class="h-4 w-24 rounded bg-white/10"></div>
        <div class="h-5 w-16 rounded bg-white/10"></div>
        <div class="h-4 w-24 rounded bg-white/10"></div>
      </div>
      <div
        class="mx-auto bg-board-dark rounded aspect-square"
        style="width: min(100%, 30rem);"
      ></div>
    </div>
    <div class="card">
      <div class="h-4 w-16 rounded bg-white/10 mb-2"></div>
      <div class="grid grid-cols-2 gap-x-4 gap-y-1">
        {#each Array(6) as _}
          <div class="h-3 rounded bg-white/5"></div>
        {/each}
      </div>
    </div>
  </div>
{:else if error}
  <div class="text-center py-12 text-primary-err">{error}</div>
{:else if waiting}
  <div class="text-center py-12 text-white/50">
    <div class="animate-pulse">Waiting for game to be indexed...</div>
  </div>
{:else if game}
  <div class="flex flex-col gap-4">
    <button
      class="text-sm text-white/60 hover:text-white self-start"
      onclick={() => goto('/')}
    >
      &larr; Back
    </button>
    <div class="flex flex-col lg:flex-row lg:items-start lg:gap-6">
      <div class="card lg:flex-1">
        <div class="flex justify-between items-center mb-2 gap-1">
          <span
            class="min-w-0 inline-flex items-center gap-1 text-sm px-2 py-1 rounded transition-all {currentTurn ===
            'White'
              ? 'bg-white/20 font-bold ring-1 ring-white/50'
              : 'text-white/60'}"
          >
            <span class="shrink-0 w-3 h-3 rounded-full bg-white"></span>
            <span class="truncate max-w-24 sm:max-w-none">
              {game.white.type === 'Human'
                ? truncateAddr(game.white.value)
                : `AI (${game.white.value})`}
            </span>
            {#if currentTurn === 'White'}
              <span class="shrink-0 text-xs text-primary-green">&#9654;</span>
            {/if}
          </span>
          <span
            class="shrink-0 text-xs sm:text-sm px-1.5 sm:px-2 py-0.5 rounded whitespace-nowrap {game.status ===
            'in_progress'
              ? 'bg-primary-bgOk text-primary-green'
              : game.status === 'finished'
                ? 'bg-white/10 text-white/50'
                : 'bg-primary-bgErr text-primary-err'}"
          >
            {#if isSpectating}
              <span class="sm:hidden">Spec.</span><span class="hidden sm:inline"
                >Spectating</span
              >
            {:else if game.status === 'in_progress'}
              <span class="sm:hidden">Live</span><span class="hidden sm:inline"
                >in progress</span
              >
            {:else if game.status === 'waiting'}
              <span class="sm:hidden">Wait</span><span class="hidden sm:inline"
                >waiting</span
              >
            {:else if game.status === 'finished'}
              <span class="sm:hidden">Done</span><span class="hidden sm:inline"
                >finished</span
              >
            {:else}
              {game.status?.replace('_', ' ') ?? 'unknown'}
            {/if}
          </span>
          <span
            class="min-w-0 inline-flex items-center gap-1 text-sm px-2 py-1 rounded transition-all {currentTurn ===
            'Black'
              ? 'bg-white/20 font-bold ring-1 ring-gray-400'
              : 'text-white/60'}"
          >
            {#if currentTurn === 'Black'}
              <span class="shrink-0 text-xs text-primary-green">&#9654;</span>
            {/if}
            <span class="truncate max-w-24 sm:max-w-none">
              {game.black?.type === 'Human'
                ? truncateAddr(game.black.value ?? '')
                : game.black?.type?.toLowerCase() === 'ai'
                  ? `AI (${game.black.value})`
                  : '...'}
            </span>
            <span
              class="shrink-0 w-3 h-3 rounded-full bg-gray-700 border border-gray-500"
            ></span>
          </span>
        </div>

        {#if game.created_at}
          <div class="text-xs text-white/40 text-center mt-1">
            Started {dayjs(game.created_at).format('lll')}
          </div>
        {/if}

        <div class="flex justify-center relative">
          <Board
            board={displayBoard}
            fen={displayFen}
            onMove={handleMove}
            disabled={game.status !== 'in_progress' ||
              submitting ||
              !indexed ||
              !isMyTurn ||
              !isViewingCurrent}
            loading={submitting}
            {flipped}
            lastMove={displayLastMove}
          />
          {#if !indexed}
            <div
              class="absolute inset-0 flex items-center justify-center bg-black/60 rounded z-10"
            >
              <div class="flex flex-col items-center gap-2">
                <div
                  class="w-6 h-6 border-2 border-white/30 border-t-white rounded-full animate-spin"
                ></div>
                <span class="text-sm text-white/70">Setting up game...</span>
              </div>
            </div>
          {/if}
        </div>

        {#if game.status === 'in_progress'}
          <div class="text-center mt-2">
            {#if isMyTurn}
              <span
                class="inline-block text-sm font-bold px-3 py-1 rounded bg-primary-bgOk text-primary-green animate-pulse"
              >
                Your turn!
              </span>
            {:else if $accountStore}
              <span class="text-sm text-white/50">
                Waiting for {currentTurn ?? 'opponent'}...
              </span>
            {:else}
              <span class="text-sm text-white/50">
                {currentTurn ?? 'Unknown'}'s turn
              </span>
            {/if}
          </div>
        {/if}

        {#if game.outcome}
          <div class="text-center mt-2 font-semibold">
            {#if game.outcome.result === 'Stalemate'}
              Draw &mdash; Stalemate
            {:else if game.resigner}
              {game.outcome.color} wins by resignation!
            {:else}
              {game.outcome.color} wins by checkmate!
            {/if}
          </div>
        {/if}

        {#if canResign || canCancel || canPublicCancel}
          <div class="flex gap-2 mt-3 justify-center">
            {#if canResign}
              <button
                class="btn text-sm text-primary-err border border-primary-err hover:bg-primary-bgErr"
                onclick={handleResign}
              >
                Resign
              </button>
            {/if}
            {#if canCancel}
              <button class="btn-secondary text-sm" onclick={handleCancel}>
                Cancel Game
              </button>
            {/if}
            {#if canPublicCancel}
              <button
                class="btn-secondary text-sm text-primary-warn border border-primary-warn/50"
                onclick={handlePublicCancel}
              >
                Cancel Stale Game
              </button>
            {/if}
          </div>
        {/if}
      </div>

      <div class="flex flex-col gap-4 lg:w-80 lg:shrink-0">
        <MoveHistory
          {moves}
          selectedMoveIndex={viewingMoveIndex}
          onSelectMove={selectMove}
          {isViewingCurrent}
          isLiveMyTurn={isMyTurn}
        />

        {#if gameBets.length > 0}
          <div class="card space-y-2">
            <h3 class="text-sm font-semibold">Game Bets ({gameBets.length})</h3>
            <div class="space-y-1.5">
              {#each gameBets as bet}
                <div class="flex items-center justify-between text-xs">
                  <div class="truncate mr-2">
                    <span class="text-white/70">{truncateAddr(bet.bettor)}</span
                    >
                    <span class="text-white/40 ml-1">bet {bet.amount} on</span>
                    <span class="text-primary ml-1"
                      >{truncateAddr(bet.winner)}</span
                    >
                  </div>
                  <div class="shrink-0 flex items-center gap-2">
                    <span
                      class="px-1.5 py-0.5 rounded {bet.status === 'pending'
                        ? 'bg-yellow-400/20 text-yellow-400'
                        : bet.status === 'locked'
                          ? 'bg-blue-400/20 text-blue-400'
                          : 'bg-green-400/20 text-green-400'}"
                      >{bet.status}</span
                    >
                    {#if bet.payout}
                      <span class="text-green-400">+{bet.payout}</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<ConfirmModal
  open={showResignModal}
  title="Resign Game?"
  message="Are you sure you want to resign? This cannot be undone and will count as a loss."
  confirmLabel="Resign"
  onconfirm={confirmResign}
  onclose={() => (showResignModal = false)}
/>

<ConfirmModal
  open={showCancelModal}
  title="Cancel Game?"
  message="Are you sure you want to cancel this game?"
  confirmLabel="Yes, Cancel"
  onconfirm={confirmCancel}
  onclose={() => (showCancelModal = false)}
/>

<ConfirmModal
  open={showPublicCancelModal}
  title="Cancel Stale Game?"
  message="This game has been inactive for over 14 days. Cancelling will refund all wagers and bets to the players. This action cannot be undone."
  confirmLabel="Cancel Stale Game"
  onconfirm={confirmPublicCancel}
  onclose={() => (showPublicCancelModal = false)}
/>
