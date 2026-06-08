<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import {
    api,
    type Game,
    type GameMove,
    type Bet,
    type GameOverview
  } from '$lib/api/client';
  import { contract, getTxLogs } from '$lib/near/connector';
  import { accountStore } from '$lib/near/account';
  import { colorFromFEN } from '$lib/chess/board';
  import { showToast } from '$lib/toast';
  import { loadGameFromContract } from '$lib/game';
  import type { GameId, ContractGameData } from '$lib/game';
  import Board from '$lib/components/Board.svelte';
  import MoveHistory from '$lib/components/MoveHistory.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';

  import dayjs from 'dayjs';

  function parseMoveNotation(
    notation: string
  ): { from: string; to: string } | null {
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

  let game = $state<Game | null>(null);
  let moves = $state<GameMove[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let submitting = $state(false);
  let gameBets = $state<Bet[]>([]);
  let pollInterval: ReturnType<typeof setInterval>;
  let showResignModal = $state(false);
  let showCancelModal = $state(false);
  let pendingLastMove = $state<{ from: string; to: string } | null>(null);
  let viewingMoveIndex = $state<number | null>(null);
  const STARTING_FEN =
    'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

  const gameIdStr = decodeURIComponent(page.params.id ?? '');
  const gameId: GameId = JSON.parse(gameIdStr);

  let lastMove = $derived(
    pendingLastMove ??
      (moves.length > 0
        ? parseMoveNotation(moves[moves.length - 1].move_notation)
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
      return parseMoveNotation(moves[viewingMoveIndex].move_notation);
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

  async function loadFromContract(): Promise<ContractGameData> {
    return loadGameFromContract(gameId);
  }

  async function load() {
    try {
      const [g, m] = await Promise.all([
        api.game(gameIdStr),
        api.gameMoves(gameIdStr)
      ]);
      if (game && game.status !== 'in_progress' && game.status !== g.status) {
        return;
      }
      game = g;
      moves = m;
      contractTurnColor = null;
      console.log(
        '[game] load() moves count:',
        m.length,
        'last move:',
        m.length > 0 ? m[m.length - 1].move_notation : 'none',
        'pendingLastMove:',
        pendingLastMove
      );
      if (m.length > 0 && pendingLastMove) {
        const parsed = parseMoveNotation(m[m.length - 1].move_notation);
        if (
          parsed &&
          parsed.from === pendingLastMove.from &&
          parsed.to === pendingLastMove.to
        ) {
          pendingLastMove = null;
        }
      } else if (m.length > 0) {
        pendingLastMove = null;
      }
      gameBets = await api.gameBets(gameIdStr).catch(() => []);
    } catch (e) {
      if (game?.status !== 'in_progress' && game?.status !== undefined) return;
      console.warn('[game] API load failed, falling back to contract:', e);
      try {
        const contractGame = await loadFromContract();
        game = { ...contractGame, moves: [] } as Game;
        contractTurnColor = contractGame.turn_color;
        moves = [];
        gameBets = [];
      } catch (e2) {
        console.error('[game] contract fallback also failed:', e2);
        error = 'Failed to load game';
      }
    } finally {
      loading = false;
    }
  }

  function parseTxLogs(logs: string[]) {
    let lastMove: { from: string; to: string } | null = null;
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
            lastMove = { from: parts[0], to: parts[1] };
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
    return { lastMove, outcome, board, resigner, cancelled };
  }

  function handleMove(from: string, to: string) {
    if (!game || submitting) return;
    submitting = true;
    const isAiGame = game.white.type === 'AI' || game.black?.type === 'AI';
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

        if (isAiGame && !parsed.lastMove) {
          const txHash = tx.transaction?.hash ?? tx.transaction_outcome?.id;
          if (txHash) {
            try {
              const rpcLogs = await getTxLogs(txHash);
              parsed = parseTxLogs(rpcLogs);
            } catch (e) {
              console.warn('[game] getTxLogs failed:', e);
            }
          }
        }

        if (parsed.outcome && parsed.board && game) {
          game = {
            ...game,
            board: parsed.board,
            fen: undefined,
            status: 'finished' as const,
            outcome: parsed.outcome as GameOverview['outcome']
          };
          pendingLastMove = null;
          contractTurnColor = null;
          retryLoadUntilFinished();
        } else {
          if (isAiGame && parsed.lastMove) {
            pendingLastMove = parsed.lastMove;
          }
          load();
        }
      })
      .catch(() => {
        submitting = false;
      });
    showToast('info', 'Submitting move...');
  }

  async function retryLoadUntilFinished(targetStatus = 'finished') {
    for (let i = 0; i < 5; i++) {
      await new Promise(r => setTimeout(r, 2000));
      try {
        const g = await api.game(gameIdStr);
        if (g.status === targetStatus) {
          await load();
          return;
        }
      } catch {
        // retry
      }
    }
  }

  function handleResign() {
    showResignModal = true;
  }

  function confirmResign() {
    if (!game) return;
    showResignModal = false;
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
          retryLoadUntilFinished();
        } else {
          showToast('success', 'Game resigned');
          load();
        }
      })
      .catch((err: unknown) => {
        showToast(
          'error',
          'Resign failed',
          err instanceof Error ? err.message : String(err)
        );
      });
  }

  function handleCancel() {
    showCancelModal = true;
  }

  function confirmCancel() {
    if (!game) return;
    showCancelModal = false;
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
          retryLoadUntilFinished('cancelled');
        } else {
          showToast('success', 'Game cancelled');
          load();
        }
      })
      .catch((err: unknown) => {
        showToast(
          'error',
          'Cancel failed',
          err instanceof Error ? err.message : String(err)
        );
      });
  }

  onMount(() => {
    load();
    pollInterval = setInterval(load, 15000);
    return () => clearInterval(pollInterval);
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
{:else if game}
  <div class="flex flex-col gap-4">
    <button
      class="text-sm text-white/60 hover:text-white self-start"
      onclick={() => goto('/')}
    >
      &larr; Back
    </button>
    <div class="card-accent">
      <div class="flex justify-between items-center mb-2">
        <span
          class="text-sm px-2 py-1 rounded transition-all {currentTurn ===
          'White'
            ? 'bg-white/20 font-bold ring-1 ring-white/50'
            : 'text-white/60'}"
        >
          <span
            class="inline-block w-3 h-3 rounded-full bg-white mr-1 align-middle"
          ></span>
          {game.white.type === 'Human'
            ? game.white.value
            : `AI (${game.white.value})`}
          {#if currentTurn === 'White'}
            <span class="text-xs ml-1 text-primary-green">&#9654;</span>
          {/if}
        </span>
        <span
          class="text-sm px-2 py-0.5 rounded {game.status === 'in_progress'
            ? 'bg-primary-bgOk text-primary-green'
            : game.status === 'finished'
              ? 'bg-white/10 text-white/50'
              : 'bg-primary-bgErr text-primary-err'}"
        >
          {#if isSpectating}
            Spectating
          {:else}
            {game.status?.replace('_', ' ') ?? 'unknown'}
          {/if}
        </span>
        <span
          class="text-sm px-2 py-1 rounded transition-all {currentTurn ===
          'Black'
            ? 'bg-white/20 font-bold ring-1 ring-gray-400'
            : 'text-white/60'}"
        >
          {#if currentTurn === 'Black'}
            <span class="text-xs mr-1 text-primary-green">&#9654;</span>
          {/if}
          {game.black?.type === 'Human'
            ? game.black.value
            : game.black?.type?.toLowerCase() === 'ai'
              ? `AI (${game.black.value})`
              : '...'}
          <span
            class="inline-block w-3 h-3 rounded-full bg-gray-700 border border-gray-500 ml-1 align-middle"
          ></span>
        </span>
      </div>

      {#if game.created_at}
        <div class="text-xs text-white/40 text-center mt-1">
          Started {dayjs(game.created_at).format('lll')}
        </div>
      {/if}

      <div class="flex justify-center">
        <Board
          board={displayBoard}
          fen={displayFen}
          onMove={handleMove}
          disabled={game.status !== 'in_progress' ||
            submitting ||
            !isMyTurn ||
            !isViewingCurrent}
          {flipped}
          lastMove={displayLastMove}
        />
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
          {game.outcome.result === 'Victory'
            ? `${game.outcome.color} wins!`
            : 'Draw - Stalemate'}
        </div>
      {/if}

      {#if canResign || canCancel}
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
        </div>
      {/if}
    </div>

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
                <span class="text-white/70"
                  >{bet.bettor.length > 20
                    ? bet.bettor.slice(0, 10) + '...' + bet.bettor.slice(-6)
                    : bet.bettor}</span
                >
                <span class="text-white/40 ml-1">bet {bet.amount} on</span>
                <span class="text-primary ml-1"
                  >{bet.winner.length > 20
                    ? bet.winner.slice(0, 10) + '...' + bet.winner.slice(-6)
                    : bet.winner}</span
                >
              </div>
              <div class="shrink-0 flex items-center gap-2">
                <span
                  class="px-1.5 py-0.5 rounded {bet.status === 'pending'
                    ? 'bg-yellow-400/20 text-yellow-400'
                    : bet.status === 'locked'
                      ? 'bg-blue-400/20 text-blue-400'
                      : 'bg-green-400/20 text-green-400'}">{bet.status}</span
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
