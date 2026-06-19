<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Bet, type BetStats } from '$lib/api/client';
  import { accountStore, isLoggedIn } from '$lib/near/account';
  import { contract } from '$lib/near/connector';
  import { showTxToast } from '$lib/toast';
  import { formatBalance } from '$lib/tokens';
  import { fetchAllMetadata, isWrapNear, WRAP_NEAR_ID } from '$lib/tokens';
  import type { FtMetadata } from '$lib/tokens';
  import NEAR_ICON from '$lib/assets/near.svg';
  import TokenInput from '$lib/components/TokenInput.svelte';
  import AccountSearch from '$lib/components/AccountSearch.svelte';
  import Pagination from '$lib/components/Pagination.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';
  import TokenBalanceList from '$lib/components/TokenBalanceList.svelte';

  const PER_PAGE = 10;

  type Tab = 'marketplace' | 'my-bets';

  let tab = $state<Tab>('marketplace');

  let newBetPlayer0 = $state('');
  let newBetPlayer1 = $state('');
  let newBetWinner = $state('');
  let newBetToken = $state('');
  let newBetTokenSymbol = $state('');
  let newBetAmount = $state('');
  let newBetRawAmount = $state('');
  let newBetInsufficientBalance = $state(false);
  let newBetSubmitting = $state(false);
  let newBetError = $state('');
  let showBetConfirm = $state(false);

  let globalBets = $state<Bet[]>([]);
  let globalBetsLoading = $state(false);
  let globalBetsHasMore = $state(false);
  let globalBetsCursor = $state<string | null>(null);
  let globalBetsCursors = $state<(string | null)[]>([null]);
  let globalBetsPage = $state(1);
  let globalBetsStatusFilter = $state<'pending' | 'locked' | 'resolved' | ''>(
    ''
  );

  let myBets = $state<Bet[]>([]);
  let myBetsLoading = $state(false);
  let myBetsHasMore = $state(false);
  let myBetsPage = $state(1);
  let statusFilter = $state<'pending' | 'locked' | 'resolved' | ''>('');
  let betStats = $state<BetStats | null>(null);
  let statsLoading = $state(true);
  let tokens = $state<string[]>([]);
  let tokenBalances = $state<Array<[string, string]>>([]);
  let metadataMap = $state<Map<string, FtMetadata>>(new Map());
  let withdrawing = $state<string | null>(null);

  function shortId(id: string): string {
    if (id.length <= 20) return id;
    return id.slice(0, 8) + '...' + id.slice(-6);
  }

  function getTokenMeta(tokenId: string): FtMetadata | undefined {
    if (isWrapNear(tokenId)) {
      return {
        decimals: 24,
        symbol: 'NEAR',
        name: 'NEAR',
        icon: NEAR_ICON,
        spec: '',
        reference: null,
        reference_hash: null
      };
    }
    return metadataMap.get(tokenId);
  }

  function formatTokenAmount(raw: string, tokenId: string): string {
    const meta = getTokenMeta(tokenId);
    if (!meta) return raw;
    return formatBalance(raw, meta.decimals);
  }

  function tokenLabel(tokenId: string): string {
    const meta = getTokenMeta(tokenId);
    return (
      meta?.symbol ??
      (tokenId.length > 16 ? tokenId.slice(0, 8) + '...' : tokenId)
    );
  }

  async function loadMetadata() {
    try {
      const tkns = await contract.getTokenWhitelist().catch(() => []);
      tokens = tkns;
      const nonWrap = tkns.filter(id => !isWrapNear(id));
      if (nonWrap.length > 0) {
        metadataMap = await fetchAllMetadata(nonWrap);
      }
    } catch {}
  }

  function requestBet() {
    if (!$accountStore) return;
    newBetError = '';
    const p0 = newBetPlayer0.trim();
    const p1 = newBetPlayer1.trim();
    if (!p0 || !p1) {
      newBetError = 'Both player addresses are required';
      return;
    }
    if (p0 === p1) {
      newBetError = 'Players must be different';
      return;
    }
    if ($accountStore === p0 || $accountStore === p1) {
      newBetError = 'You cannot bet on your own game';
      return;
    }
    if (!newBetWinner) {
      newBetError = 'Select a winner';
      return;
    }
    if (!newBetRawAmount || Number(newBetAmount) <= 0) {
      newBetError = 'Enter a valid amount';
      return;
    }
    if (!newBetToken) {
      newBetError = 'Select a token';
      return;
    }
    if (newBetInsufficientBalance) {
      newBetError = 'Insufficient balance';
      return;
    }
    showBetConfirm = true;
  }

  function doPlaceBet() {
    if (!$accountStore) return;
    showBetConfirm = false;
    const p0 = newBetPlayer0.trim();
    const p1 = newBetPlayer1.trim();
    const players: [string, string] = [p0, p1].sort() as [string, string];
    newBetSubmitting = true;
    const promise = contract.placeBet(
      newBetToken,
      players,
      newBetWinner,
      newBetRawAmount
    );
    showTxToast(promise);
    promise
      .then(() => {
        newBetPlayer0 = '';
        newBetPlayer1 = '';
        newBetWinner = '';
        newBetAmount = '';
        newBetRawAmount = '';
        newBetError = '';
        setTimeout(() => loadGlobalBets(true), 4000);
      })
      .catch(() => {})
      .finally(() => {
        newBetSubmitting = false;
      });
  }

  async function loadStats() {
    if (!$accountStore) return;
    try {
      const [s, tkns, bals] = await Promise.all([
        api.betStats($accountStore).catch(() => null),
        contract.getTokenWhitelist().catch(() => []),
        contract.getTokens($accountStore).catch(() => [])
      ]);
      betStats = s;
      tokens = tkns;
      tokenBalances = bals;
    } catch (e) {
      console.error('Failed to load bet stats:', e);
    } finally {
      statsLoading = false;
    }
  }

  async function loadMyBets(p: number) {
    if (!$accountStore) return;
    myBetsLoading = true;
    try {
      const offset = (p - 1) * PER_PAGE;
      const result = await api.bets(
        $accountStore,
        statusFilter || undefined,
        undefined,
        offset + PER_PAGE + 1
      );
      const allItems = result.items;
      myBetsHasMore = allItems.length > offset + PER_PAGE;
      myBets = allItems.slice(offset, offset + PER_PAGE);
      myBetsPage = p;
    } catch (e) {
      console.error('Failed to load bets:', e);
    } finally {
      myBetsLoading = false;
    }
  }

  function filterByStatus(status: 'pending' | 'locked' | 'resolved' | '') {
    statusFilter = status;
    myBetsPage = 1;
    loadMyBets(1);
  }

  async function loadGlobalBets(reset = false) {
    globalBetsLoading = true;
    try {
      if (reset) {
        globalBetsCursors = [null];
        globalBetsPage = 1;
      }
      const cursor = globalBetsCursors[globalBetsPage - 1] ?? undefined;
      const result = await api.globalBets(
        globalBetsStatusFilter || undefined,
        cursor,
        PER_PAGE
      );
      globalBets = result.items;
      const nextCursor = result.next_cursor;
      if (nextCursor && globalBetsPage >= globalBetsCursors.length) {
        globalBetsCursors = [...globalBetsCursors, nextCursor];
      }
      globalBetsHasMore = nextCursor !== null;
      globalBetsCursor = nextCursor;
    } catch (e) {
      console.error('Failed to load global bets:', e);
    } finally {
      globalBetsLoading = false;
    }
  }

  function goToGlobalBetsPage(p: number) {
    const totalPages = globalBetsHasMore ? globalBetsPage + 1 : globalBetsPage;
    if (p < 1 || p > totalPages) return;
    globalBetsPage = p;
    loadGlobalBets();
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function filterGlobalBets(status: 'pending' | 'locked' | 'resolved' | '') {
    globalBetsStatusFilter = status;
    loadGlobalBets(true);
  }

  function handleWithdraw(tokenId: string) {
    if (!$accountStore) return;
    withdrawing = tokenId;
    showTxToast(
      contract.withdrawToken(tokenId).finally(() => {
        withdrawing = null;
        setTimeout(loadStats, 4000);
      })
    );
  }

  function switchTab(t: Tab) {
    tab = t;
    if (t === 'marketplace' && globalBets.length === 0) loadGlobalBets(true);
    if (t === 'my-bets' && myBets.length === 0) {
      loadMyBets(1);
      loadStats();
    }
  }

  onMount(async () => {
    if ($isLoggedIn) {
      loadMetadata();
      loadGlobalBets(true);
      loadStats();
    }
  });
</script>

<svelte:head>
  <title>Protocol Pawns - Bets</title>
</svelte:head>

{#if !$isLoggedIn}
  <div class="text-center py-12 text-white/50">
    Connect your wallet to view bets
  </div>
{:else}
  <div class="space-y-4">
    <h2 class="text-xl font-bold text-primary text-center">Bets</h2>
    <div class="flex gap-2 justify-center">
      <button
        class="btn text-xs"
        class:btn-primary={tab === 'marketplace'}
        onclick={() => switchTab('marketplace')}
      >
        Marketplace
      </button>
      <button
        class="btn text-xs"
        class:btn-primary={tab === 'my-bets'}
        onclick={() => switchTab('my-bets')}
      >
        My Bets
      </button>
    </div>

    {#if tab === 'marketplace'}
      <div class="card space-y-3">
        <h2 class="text-base font-semibold">Place a Bet</h2>

        {#if newBetError}
          <p class="text-xs text-red-400">{newBetError}</p>
        {/if}

        <div class="flex gap-2">
          <div class="flex-1">
            <label class="text-xs text-white/40 mb-1 block">Player 1</label>
            <AccountSearch bind:value={newBetPlayer0} />
          </div>
          <div class="flex-1">
            <label class="text-xs text-white/40 mb-1 block">Player 2</label>
            <AccountSearch bind:value={newBetPlayer1} />
          </div>
        </div>

        {#if newBetPlayer0.trim() && newBetPlayer1.trim() && newBetPlayer0.trim() !== newBetPlayer1.trim()}
          <label class="text-xs text-white/40 block">Who will win?</label>
          <div class="flex gap-2">
            <button
              class="flex-1 rounded px-3 py-2 text-xs font-medium border transition-all {newBetWinner ===
              newBetPlayer0.trim()
                ? 'bg-primary/20 border-primary text-primary'
                : 'bg-white/5 border-white/15 text-white/60 hover:border-white/30'}"
              onclick={() => (newBetWinner = newBetPlayer0.trim())}
            >
              {shortId(newBetPlayer0.trim())} wins
            </button>
            <button
              class="flex-1 rounded px-3 py-2 text-xs font-medium border transition-all {newBetWinner ===
              newBetPlayer1.trim()
                ? 'bg-primary/20 border-primary text-primary'
                : 'bg-white/5 border-white/15 text-white/60 hover:border-white/30'}"
              onclick={() => (newBetWinner = newBetPlayer1.trim())}
            >
              {shortId(newBetPlayer1.trim())} wins
            </button>
          </div>
        {/if}

        {#if newBetWinner}
          <TokenInput
            bind:tokenId={newBetToken}
            bind:tokenSymbol={newBetTokenSymbol}
            bind:amount={newBetAmount}
            bind:rawAmount={newBetRawAmount}
            bind:insufficientBalance={newBetInsufficientBalance}
          />

          <button
            class="btn-primary text-sm w-full"
            disabled={newBetSubmitting ||
              !newBetRawAmount ||
              !newBetToken ||
              newBetInsufficientBalance}
            onclick={requestBet}
          >
            {newBetSubmitting ? 'Placing...' : 'Place Bet'}
          </button>
        {/if}
      </div>

      <div class="space-y-3">
        <div class="flex items-center justify-between">
          <h2 class="text-base font-semibold">Recent Bets</h2>
          <div class="flex gap-1">
            <button
              class="btn-secondary text-xs px-2 py-0.5"
              class:btn-primary={globalBetsStatusFilter === ''}
              onclick={() => filterGlobalBets('')}
            >
              All
            </button>
            <button
              class="btn-secondary text-xs px-2 py-0.5"
              class:btn-primary={globalBetsStatusFilter === 'pending'}
              onclick={() => filterGlobalBets('pending')}
            >
              Pending
            </button>
            <button
              class="btn-secondary text-xs px-2 py-0.5"
              class:btn-primary={globalBetsStatusFilter === 'locked'}
              onclick={() => filterGlobalBets('locked')}
            >
              Locked
            </button>
            <button
              class="btn-secondary text-xs px-2 py-0.5"
              class:btn-primary={globalBetsStatusFilter === 'resolved'}
              onclick={() => filterGlobalBets('resolved')}
            >
              Resolved
            </button>
          </div>
        </div>

        {#if globalBetsLoading && globalBets.length === 0}
          <div class="space-y-2 animate-pulse">
            {#each Array(5) as _}
              <div class="card">
                <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
                <div class="h-3 w-1/3 rounded bg-white/5"></div>
              </div>
            {/each}
          </div>
        {:else if globalBets.length === 0}
          <p class="text-white/50 text-sm">No bets found</p>
        {:else}
          <div class="space-y-2">
            {#each globalBets as bet}
              <div class="card flex items-center justify-between">
                <div>
                  <div class="font-medium text-sm">
                    <span class="text-white/70">{shortId(bet.bettor)}</span>
                    <span class="text-white/40 mx-1">bet on</span>
                    <span class="text-primary">{shortId(bet.winner)}</span>
                  </div>
                  <div class="text-xs text-white/50">
                    {formatTokenAmount(bet.amount, bet.token_id)}
                    {tokenLabel(bet.token_id)}
                  </div>
                </div>
                <div class="text-right">
                  <div
                    class="text-xs {bet.status === 'pending'
                      ? 'text-yellow-400'
                      : bet.status === 'locked'
                        ? 'text-blue-400'
                        : 'text-green-400'}"
                  >
                    {bet.status}
                  </div>
                  {#if bet.payout}
                    <div class="text-xs text-green-400">+{bet.payout}</div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>

          {#if globalBetsHasMore || globalBetsPage > 1}
            <Pagination
              page={globalBetsPage}
              totalPages={globalBetsHasMore
                ? globalBetsPage + 1
                : globalBetsPage}
              onchange={goToGlobalBetsPage}
            />
          {/if}
        {/if}
      </div>
    {:else if tab === 'my-bets'}
      <h2 class="text-base font-semibold">My Bets</h2>

      {#if betStats && !statsLoading}
        <div class="grid grid-cols-2 gap-3">
          <div class="text-center bg-primary-transparent2 rounded p-2">
            <div class="text-lg font-bold text-primary">
              {betStats.total_bets}
            </div>
            <div class="text-xs text-white/50">Total</div>
          </div>
          <div class="text-center bg-primary-transparent2 rounded p-2">
            <div class="text-lg font-bold text-primary-green">
              {betStats.won_bets}
            </div>
            <div class="text-xs text-white/50">Won</div>
          </div>
        </div>
        {#if Object.keys(betStats.by_token).length > 0}
          <div class="card space-y-1">
            <h3 class="text-sm font-semibold">Wagered / Earned by Token</h3>
            {#each Object.entries(betStats.by_token) as [tokenId, ts]}
              <div class="flex items-center justify-between text-sm">
                <span class="text-white/70 truncate mr-2"
                  >{tokenLabel(tokenId)}</span
                >
                <div class="flex items-center gap-3 shrink-0">
                  <span class="text-primary-warn"
                    >{formatTokenAmount(ts.wagered, tokenId)}</span
                  >
                  <span class="text-white/30">/</span>
                  <span class="text-primary-green"
                    >{formatTokenAmount(ts.won, tokenId)}</span
                  >
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}

      {#if tokenBalances.length > 0}
        <div class="card space-y-2">
          <h3 class="text-sm font-semibold">Token Balances</h3>
          <TokenBalanceList
            tokens={tokenBalances}
            showWithdraw
            onWithdraw={handleWithdraw}
            {withdrawing}
          />
        </div>
      {/if}

      <div class="flex gap-2">
        <button
          class="btn-secondary text-xs"
          class:btn-primary={statusFilter === ''}
          onclick={() => filterByStatus('')}
        >
          All
        </button>
        <button
          class="btn-secondary text-xs"
          class:btn-primary={statusFilter === 'pending'}
          onclick={() => filterByStatus('pending')}
        >
          Pending
        </button>
        <button
          class="btn-secondary text-xs"
          class:btn-primary={statusFilter === 'locked'}
          onclick={() => filterByStatus('locked')}
        >
          Locked
        </button>
        <button
          class="btn-secondary text-xs"
          class:btn-primary={statusFilter === 'resolved'}
          onclick={() => filterByStatus('resolved')}
        >
          Resolved
        </button>
      </div>

      {#if myBetsLoading}
        <div class="space-y-2 animate-pulse">
          {#each Array(3) as _}
            <div class="card">
              <div class="h-4 w-2/3 rounded bg-white/10 mb-1"></div>
              <div class="h-3 w-1/3 rounded bg-white/5"></div>
            </div>
          {/each}
        </div>
      {:else if myBets.length === 0}
        <p class="text-white/50 text-sm">No bets found</p>
      {:else}
        <div class="space-y-2">
          {#each myBets as bet}
            <div class="card flex items-center justify-between">
              <div>
                <div class="font-medium text-sm">
                  <span class="text-white/70">You bet on</span>
                  <span class="text-primary ml-1">{shortId(bet.winner)}</span>
                </div>
                <div class="text-xs text-white/50">
                  {formatTokenAmount(bet.amount, bet.token_id)}
                  {tokenLabel(bet.token_id)}
                </div>
              </div>
              <div class="text-right">
                <div
                  class="text-xs {bet.status === 'pending'
                    ? 'text-yellow-400'
                    : bet.status === 'locked'
                      ? 'text-blue-400'
                      : 'text-green-400'}"
                >
                  {bet.status}
                </div>
                {#if bet.payout}
                  <div class="text-xs text-green-400">+{bet.payout}</div>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}

      {#if !myBetsLoading && (myBetsPage > 1 || myBetsHasMore)}
        <Pagination
          page={myBetsPage}
          totalPages={myBetsHasMore ? myBetsPage + 1 : myBetsPage}
          onchange={p => {
            loadMyBets(p);
            window.scrollTo({ top: 0, behavior: 'smooth' });
          }}
        />
      {/if}
    {/if}
  </div>
{/if}

<ConfirmModal
  open={showBetConfirm}
  title="Place Bet?"
  message="Bet {newBetAmount} {newBetTokenSymbol} on {shortId(
    newBetWinner
  )} to win?"
  confirmLabel="Place Bet"
  confirmClass="btn-primary text-sm"
  onconfirm={doPlaceBet}
  onclose={() => (showBetConfirm = false)}
/>
