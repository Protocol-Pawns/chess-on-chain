<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type AccountSearchResult } from '$lib/api/client';

  let {
    value = $bindable(''),
    disabled = false,
    onenter = () => {}
  } = $props();

  let inputEl: HTMLInputElement | undefined = $state();
  let results = $state<AccountSearchResult[]>([]);
  let open = $state(false);
  let loading = $state(false);
  let focusedIndex = $state(-1);
  let inputValue = $state(value || '');
  let blurTimer: ReturnType<typeof setTimeout> | undefined = undefined;

  let debounceTimer: ReturnType<typeof setTimeout> | undefined = undefined;
  let currentQuery = $state('');

  onMount(() => {
    inputValue = value || '';
  });

  function handleInput() {
    const q = inputValue.trim();
    value = q;

    if (debounceTimer) clearTimeout(debounceTimer);

    if (q.length < 2) {
      results = [];
      open = false;
      loading = false;
      return;
    }

    loading = true;
    debounceTimer = setTimeout(async () => {
      try {
        currentQuery = q;
        const r = await api.searchAccounts(q);
        if (currentQuery === q) {
          results = r.sort((a, b) => (b.elo ?? 0) - (a.elo ?? 0));
          open = r.length > 0;
          focusedIndex = -1;
        }
      } catch {
        if (currentQuery === q) {
          results = [];
          open = false;
        }
      } finally {
        if (currentQuery === q) {
          loading = false;
        }
      }
    }, 500);
  }

  function select(accountId: string) {
    value = accountId;
    inputValue = accountId;
    open = false;
    results = [];
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      focusedIndex = Math.min(focusedIndex + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      focusedIndex = Math.max(focusedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      if (focusedIndex >= 0 && focusedIndex < results.length) {
        e.preventDefault();
        select(results[focusedIndex].account_id);
      } else if (!open) {
        onenter();
      }
    } else if (e.key === 'Escape') {
      open = false;
    }
  }

  function handleBlur() {
    blurTimer = setTimeout(() => {
      open = false;
    }, 200);
  }

  function handleFocus() {
    if (blurTimer) clearTimeout(blurTimer);
    if (results.length > 0) open = true;
  }

  function matchParts(
    text: string
  ): Array<{ text: string; highlight: boolean }> {
    const q = inputValue.trim();
    if (!q) return [{ text, highlight: false }];
    const idx = text.toLowerCase().indexOf(q.toLowerCase());
    if (idx === -1) return [{ text, highlight: false }];
    const parts: Array<{ text: string; highlight: boolean }> = [];
    if (idx > 0) parts.push({ text: text.slice(0, idx), highlight: false });
    parts.push({ text: text.slice(idx, idx + q.length), highlight: true });
    if (idx + q.length < text.length)
      parts.push({ text: text.slice(idx + q.length), highlight: false });
    return parts;
  }
</script>

<div class="relative">
  <input
    bind:this={inputEl}
    type="text"
    bind:value={inputValue}
    oninput={handleInput}
    onkeydown={handleKeydown}
    onblur={handleBlur}
    onfocus={handleFocus}
    placeholder="wallet.near"
    class="flex-1 w-full bg-transparent border border-white/15 rounded px-2 py-1.5 text-sm focus:outline-none focus:border-primary"
    {disabled}
    autocomplete="off"
  />

  {#if open || (loading && inputValue.trim().length >= 2)}
    <div
      class="absolute z-50 left-0 right-0 mt-1 bg-[#1a1a2e] border border-white/15 rounded shadow-lg overflow-hidden"
      onmouseenter={() => {
        if (blurTimer) clearTimeout(blurTimer);
      }}
      onmouseleave={() => {
        blurTimer = setTimeout(() => {
          open = false;
        }, 200);
      }}
    >
      {#if loading && results.length === 0}
        <div class="px-3 py-2 text-xs text-white/40">Searching...</div>
      {:else}
        {#each results as result, i}
          <button
            type="button"
            class="w-full flex items-center gap-3 px-3 py-2 text-left text-sm transition-colors {i ===
            focusedIndex
              ? 'bg-primary/20 border border-primary/30'
              : 'hover:bg-white/5 border border-transparent'}"
            onmousedown={() => select(result.account_id)}
            onmouseenter={() => (focusedIndex = i)}
          >
            <span class="flex-1 min-w-0 truncate font-mono">
              {#each matchParts(result.account_id) as part}
                {#if part.highlight}
                  <span class="text-primary font-semibold">{part.text}</span>
                {:else}
                  {part.text}
                {/if}
              {/each}
            </span>
            <span class="text-xs text-white/40 tabular-nums whitespace-nowrap">
              {result.elo != null ? `${result.elo} ELO` : '—'}
            </span>
            <span class="text-xs text-white/30 tabular-nums whitespace-nowrap">
              {result.wins}W {result.losses}L {result.draws}D
            </span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>
