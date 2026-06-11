<script lang="ts">
  import dayjs from 'dayjs';
  import type { Challenge, GameOverview } from '$lib/api/client';
  import { colorFromFEN } from '$lib/chess/board';
  import { truncateAddr } from '$lib/format';
  import { MAX_OPEN_GAMES, gameUrl } from '$lib/game';
  import { accountStore } from '$lib/near/account';

  interface Props {
    challenge: Challenge;
    currentAccount: string;
    gameCount?: number;
    game?: GameOverview | null;
    onaccept?: (challenge: Challenge) => void;
    onreject?: (challenge: Challenge) => void;
    oncancel?: (challenge: Challenge) => void;
  }

  let {
    challenge,
    currentAccount,
    gameCount = 0,
    game = null,
    onaccept,
    onreject,
    oncancel
  }: Props = $props();

  let isIncoming = $derived(challenge.challenger !== currentAccount);
  let otherAccount = $derived(
    isIncoming ? challenge.challenger : challenge.challenged
  );
  let isPendingIncoming = $derived(
    challenge.status === 'pending' && challenge.challenged === currentAccount
  );
  let isPendingOutgoing = $derived(
    challenge.status === 'pending' && challenge.challenger === currentAccount
  );
  let isAccepted = $derived(
    challenge.status === 'accepted' && challenge.game_id
  );
  let atMaxGames = $derived(gameCount >= MAX_OPEN_GAMES);

  let turn = $derived(
    game && game.status === 'in_progress' && game.fen
      ? colorFromFEN(game.fen)
      : null
  );

  let isMyTurn = $derived.by(() => {
    if (!turn || !$accountStore || !game) return false;
    const myColor =
      game.white.value === $accountStore
        ? 'White'
        : game.black?.value === $accountStore
          ? 'Black'
          : null;
    return turn === myColor;
  });
</script>

{#if isAccepted && game}
  <a
    href={gameUrl(JSON.parse(challenge.game_id!))}
    class={isMyTurn ? 'card-accent block' : 'card-hover block'}
  >
    <div class="flex justify-between items-start mb-1">
      <div class="text-sm space-y-0.5">
        <div class="font-medium">
          <span
            class="inline-block w-3 h-3 rounded-full bg-white mr-1 align-middle"
          ></span>
          {game.white.type === 'Human' ? truncateAddr(game.white.value) : 'AI'}
        </div>
        <div>
          <span
            class="inline-block w-3 h-3 rounded-full bg-gray-700 border border-gray-500 mr-1 align-middle"
          ></span>
          {game.black?.type === 'Human'
            ? truncateAddr(game.black.value ?? '')
            : game.black?.type?.toLowerCase() === 'ai'
              ? `AI (${game.black.value})`
              : 'Waiting...'}
        </div>
      </div>
      <span
        class="text-xs {game.status === 'in_progress'
          ? 'text-primary-green'
          : game.status === 'finished'
            ? 'text-white/50'
            : 'text-primary-err'}"
      >
        {game.status === 'in_progress' ? 'Live' : game.status}
      </span>
    </div>
    <div class="flex justify-between items-center text-xs text-white/40">
      <span>
        {#if game.outcome}
          <span class="text-white/60">
            {#if game.outcome.result === 'Stalemate'}
              Draw
            {:else if game.resigner}
              {game.outcome.color} wins (resign)
            {:else}
              {game.outcome.color} wins (checkmate)
            {/if}
          </span>
        {:else if isMyTurn}
          <span
            class="inline-block text-xs font-bold px-2 py-0.5 rounded bg-primary-bgOk text-primary-green animate-pulse"
          >
            Your turn!
          </span>
        {:else if turn}
          {turn}'s turn
        {/if}
      </span>
      <span>
        {#if challenge.wager_token && challenge.wager_amount}
          <span class="text-yellow-400 mr-2"
            >Wager: {challenge.wager_amount}</span
          >
        {/if}
        {#if challenge.created_at}
          {dayjs(challenge.created_at).format('lll')}
        {/if}
      </span>
    </div>
  </a>
{:else}
  <div class="card flex items-center justify-between">
    <div>
      <div class="font-medium text-sm">
        {isIncoming ? '←' : '→'}
        <a
          href="/profile/{otherAccount}"
          class="hover:text-primary transition-colors"
        >
          {truncateAddr(otherAccount)}
        </a>
      </div>
      <div class="text-xs text-white/50">
        {challenge.status}
        {#if challenge.wager_token && challenge.wager_amount}
          <span class="text-yellow-400 ml-1">
            Wager: {challenge.wager_amount}
          </span>
        {/if}
      </div>
    </div>
    <div class="flex items-center gap-3">
      {#if challenge.created_at}
        <span class="text-xs text-white/40">
          {dayjs(challenge.created_at).format('lll')}
        </span>
      {/if}
      <div class="flex gap-2">
        {#if isPendingIncoming}
          {#if onaccept}
            <button
              class="btn-primary text-xs"
              onclick={() => onaccept(challenge)}
              disabled={atMaxGames}
              title={atMaxGames ? 'Max games reached' : ''}>Accept</button
            >
          {/if}
          {#if onreject}
            <button
              class="btn-secondary text-xs"
              onclick={() => onreject(challenge)}>Reject</button
            >
          {/if}
        {/if}
        {#if isPendingOutgoing && oncancel}
          <button
            class="btn-secondary text-xs"
            onclick={() => oncancel(challenge)}>Cancel</button
          >
        {/if}
        {#if isAccepted}
          <a
            href={gameUrl(JSON.parse(challenge.game_id!))}
            class="btn-primary text-xs">View Game</a
          >
        {/if}
      </div>
    </div>
  </div>
{/if}
