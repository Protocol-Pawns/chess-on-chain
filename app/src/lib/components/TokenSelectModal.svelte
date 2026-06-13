<script lang="ts">
  import Modal from './Modal.svelte';
  import type { FtMetadata } from '$lib/tokens';
  import { formatBalance } from '$lib/tokens';

  interface TokenEntry {
    id: string;
    metadata?: FtMetadata;
    balance?: string;
    loading?: boolean;
  }

  interface Props {
    open?: boolean;
    tokens: TokenEntry[];
    selectedId?: string;
    onselect: (tokenId: string) => void;
    onclose: () => void;
  }

  let {
    open = $bindable(false),
    tokens,
    selectedId = '',
    onselect,
    onclose
  }: Props = $props();
  let search = $state('');

  let filtered = $derived(
    tokens.filter(t => {
      if (!search.trim()) return true;
      const q = search.toLowerCase();
      const id = t.id.toLowerCase();
      const sym = t.metadata?.symbol?.toLowerCase() ?? '';
      const name = t.metadata?.name?.toLowerCase() ?? '';
      return id.includes(q) || sym.includes(q) || name.includes(q);
    })
  );
</script>

<Modal {open} {onclose}>
  <div class="card w-full max-w-md mx-4 bg-surface max-h-[80vh] flex flex-col">
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-base font-semibold">Select Token</h3>
      <button
        class="text-white/40 hover:text-white/70 text-lg leading-none"
        onclick={onclose}
      >
        &times;
      </button>
    </div>

    <input
      type="text"
      bind:value={search}
      placeholder="Search by name, symbol, or contract..."
      class="w-full bg-white/5 border border-white/15 rounded px-3 py-2 text-sm mb-3 focus:outline-none focus:border-primary"
    />

    <div class="flex-1 overflow-y-auto space-y-1 min-h-0">
      {#if filtered.length === 0}
        <p class="text-sm text-white/40 text-center py-4">No tokens found</p>
      {:else}
        {#each filtered as token (token.id)}
          <button
            class="w-full flex items-center gap-3 px-3 py-2.5 rounded text-left transition-colors {token.id ===
            selectedId
              ? 'bg-primary/20 border border-primary/30'
              : 'hover:bg-white/5 border border-transparent'}"
            onclick={() => {
              onselect(token.id);
              onclose();
            }}
          >
            {#if token.metadata?.icon}
              <img
                src={token.metadata.icon}
                alt={token.metadata.symbol}
                class="w-7 h-7 rounded-full shrink-0"
                loading="lazy"
              />
            {:else}
              <div
                class="w-7 h-7 rounded-full bg-white/10 flex items-center justify-center text-xs font-bold shrink-0"
              >
                {(token.metadata?.symbol ?? token.id).slice(0, 2).toUpperCase()}
              </div>
            {/if}

            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium truncate">
                  {token.metadata?.symbol ?? token.id}
                </span>
                {#if token.metadata?.name && token.metadata.symbol}
                  <span class="text-xs text-white/40 truncate">
                    {token.metadata.name}
                  </span>
                {/if}
              </div>
              <span class="text-xs text-white/30 block truncate">
                {token.id}
              </span>
            </div>

            <div class="text-right shrink-0">
              {#if token.loading}
                <span class="text-xs text-white/30">...</span>
              {:else if token.balance !== undefined}
                <span class="text-sm tabular-nums">
                  {formatBalance(token.balance, token.metadata?.decimals ?? 0)}
                </span>
              {/if}
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</Modal>
