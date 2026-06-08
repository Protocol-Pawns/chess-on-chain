<script lang="ts">
  import type { Challenge } from '$lib/api/client';
  import { truncateAddr } from '$lib/format';
  import { MAX_OPEN_GAMES } from '$lib/game';

  interface Props {
    challenge: Challenge;
    currentAccount: string;
    gameCount?: number;
    onaccept?: (challenge: Challenge) => void;
    onreject?: (challenge: Challenge) => void;
    oncancel?: (challenge: Challenge) => void;
  }

  let {
    challenge,
    currentAccount,
    gameCount = 0,
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
</script>

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
      <button class="btn-secondary text-xs" onclick={() => oncancel(challenge)}
        >Cancel</button
      >
    {/if}
    {#if isAccepted}
      <a
        href="/game/{encodeURIComponent(challenge.game_id!)}"
        class="btn-primary text-xs">View Game</a
      >
    {/if}
  </div>
</div>
