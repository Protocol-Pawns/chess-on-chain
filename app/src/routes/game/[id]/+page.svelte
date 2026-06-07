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
  import { showTxToast, showToast } from '$lib/toast';
  import { loadGameFromContract, normalizePlayer } from '$lib/game';
  import type { GameId, ContractGameData } from '$lib/game';
  import Board from '$lib/components/Board.svelte';
  import MoveHistory from '$lib/components/MoveHistory.svelte';
  import BetPanel from '$lib/components/BetPanel.svelte';

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

  const gameIdStr = decodeURIComponent(page.params.id ?? '');
  const gameId: GameId = JSON.parse(gameIdStr);

  let lastMove = $derived(
    pendingLastMove ??
      (moves.length > 0
        ? {
            from: moves[moves.length - 1].move_notation.slice(0, 2),
            to: moves[moves.length - 1].move_notation.slice(2, 4)
          }
        : null)
  );

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
      game = g;
      moves = m;
      contractTurnColor = null;
      console.log('[game] load() moves count:', m.length, 'last move:', m.length > 0 ? m[m.length - 1].move_notation : 'none', 'pendingLastMove:', pendingLastMove);
      if (m.length > 0 && pendingLastMove) {
        const apiLast = m[m.length - 1].move_notation;
        if (
          apiLast.slice(0, 2) === pendingLastMove.from &&
          apiLast.slice(2, 4) === pendingLastMove.to
        ) {
          pendingLastMove = null;
        }
      } else if (m.length > 0) {
        pendingLastMove = null;
      }
      gameBets = await api.gameBets(gameIdStr).catch(() => []);
    } catch (e) {
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

  function parseLastAiMove(
    logs: string[]
  ): { from: string; to: string } | null {
    let result: { from: string; to: string } | null = null;
    for (const log of logs) {
      try {
        const json = log.startsWith('EVENT_JSON:') ? log.slice(11) : log;
        const event = JSON.parse(json);
        if (event.standard === 'chess-game' && event.event === 'play_move') {
          const parts: string[] = event.data.mv.split(' to ');
          if (parts.length === 2) {
            result = { from: parts[0], to: parts[1] };
          }
        }
      } catch {
        // skip
      }
    }
    return result;
  }

  function handleMove(from: string, to: string) {
    if (!game || submitting) return;
    submitting = true;
    const isAiGame = game.white.type === 'AI' || game.black?.type === 'AI';
    contract
      .playMove($state.snapshot(game.game_id), from + to)
      .then(async txResult => {
        submitting = false;
        if (isAiGame) {
          let lastAiMove: { from: string; to: string } | null = null;
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
          lastAiMove = parseLastAiMove(txLogs);
          console.log('[game] txLogs from result:', txLogs, 'lastAiMove:', lastAiMove);
          if (!lastAiMove) {
            const txHash = tx.transaction?.hash ?? tx.transaction_outcome?.id;
            console.log('[game] txHash:', txHash);
            if (txHash) {
              try {
                const rpcLogs = await getTxLogs(txHash);
                console.log('[game] rpcLogs:', rpcLogs);
                lastAiMove = parseLastAiMove(rpcLogs);
                console.log('[game] lastAiMove from rpc:', lastAiMove);
              } catch (e) {
                console.warn('[game] getTxLogs failed:', e);
              }
            }
          }
          if (lastAiMove) {
            pendingLastMove = lastAiMove;
            console.log('[game] set pendingLastMove:', pendingLastMove);
          } else {
            console.log('[game] NO lastAiMove found');
          }
        } else {
          console.log('[game] not AI game, isAiGame:', isAiGame);
        }
        load();
      })
      .catch(() => {
        submitting = false;
      });
    showToast('info', 'Submitting move...');
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
      .then(() => {
        showToast('success', 'Game resigned');
        setTimeout(() => goto('/'), 1500);
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
      .then(() => {
        showToast('success', 'Game cancelled');
        setTimeout(() => goto('/'), 1500);
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
    <button class="text-sm text-white/60 hover:text-white self-start" onclick={() => goto('/')}>
      &larr; Back
    </button>
    <div class="card">
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
          {game.white.type === 'Human' ? game.white.value : `AI (${game.white.value})`}
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

      <div class="flex justify-center">
        <Board
          board={game.board}
          fen={game.fen ?? undefined}
          onMove={handleMove}
          disabled={game.status !== 'in_progress' || submitting || !isMyTurn}
          {flipped}
          {lastMove}
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
              class="btn text-sm text-primary-err border-primary-err hover:bg-primary-bgErr"
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

    <MoveHistory {moves} />

    {#if game.white.type === 'Human' && game.black?.type === 'Human'}
      <BetPanel
        playerWhite={game.white.value}
        playerBlack={game.black.value}
        disabled={game.status !== 'in_progress'}
      />
    {/if}

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

{#if showResignModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
    onclick={() => (showResignModal = false)}
  >
    <div
      class="card max-w-sm w-full mx-4 bg-[#1a1a2e]"
      onclick={e => e.stopPropagation()}
    >
      <h3 class="text-base font-semibold mb-2">Resign Game?</h3>
      <p class="text-sm text-white/70 mb-4">
        Are you sure you want to resign? This cannot be undone and will count as
        a loss.
      </p>
      <div class="flex gap-2 justify-end">
        <button
          class="btn-secondary text-sm"
          onclick={() => (showResignModal = false)}
        >
          Cancel
        </button>
        <button
          class="btn text-sm text-white bg-primary-err hover:bg-primary-err/80"
          onclick={confirmResign}
        >
          Resign
        </button>
      </div>
    </div>
  </div>
{/if}

{#if showCancelModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
    onclick={() => (showCancelModal = false)}
  >
    <div
      class="card max-w-sm w-full mx-4 bg-[#1a1a2e]"
      onclick={e => e.stopPropagation()}
    >
      <h3 class="text-base font-semibold mb-2">Cancel Game?</h3>
      <p class="text-sm text-white/70 mb-4">
        Are you sure you want to cancel this game?
      </p>
      <div class="flex gap-2 justify-end">
        <button
          class="btn-secondary text-sm"
          onclick={() => (showCancelModal = false)}
        >
          No
        </button>
        <button
          class="btn text-sm text-white bg-primary-err hover:bg-primary-err/80"
          onclick={confirmCancel}
        >
          Yes, Cancel
        </button>
      </div>
    </div>
  </div>
{/if}
