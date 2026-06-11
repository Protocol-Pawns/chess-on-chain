<script lang="ts">
  import { onMount } from 'svelte';
  import { contract } from '$lib/near/connector';
  import {
    getTokenBalance,
    getCombinedNearBalance,
    fetchAllMetadata,
    filterAllowedCharacters,
    getFormattedNumber,
    toFixedNumber,
    formatBalance,
    isWrapNear,
    WRAP_NEAR_ID
  } from '$lib/tokens';
  import type { FtMetadata } from '$lib/tokens';
  import NEAR_ICON from '$lib/assets/near.svg';
  import TokenSelectModal from './TokenSelectModal.svelte';
  import { getTokenPrice, estimateUsd } from '$lib/prices';
  const NEAR_RESERVE = 500_000_000_000_000_000_000_000n;

  let {
    tokenId = $bindable(''),
    tokenSymbol = $bindable(''),
    amount = $bindable(''),
    rawAmount = $bindable(''),
    insufficientBalance = $bindable(false),
    disabled = false
  } = $props();

  let modalOpen = $state(false);
  let tokens = $state<string[]>([]);
  let metadataMap = $state<Map<string, FtMetadata>>(new Map());
  let balances = $state<Map<string, string>>(new Map());
  let loading = $state(true);
  let accountId: string | undefined = undefined;
  let inputEl: HTMLInputElement | undefined = $state();
  let usdValue = $state<string | undefined>(undefined);

  $effect(() => {
    if (tokenId && amount) {
      getTokenPrice(tokenId, currentMeta?.symbol).then(p => {
        usdValue = estimateUsd(amount, p);
      });
    } else {
      usdValue = undefined;
    }
  });

  let isNearToken = $derived(isWrapNear(tokenId));

  let currentMeta = $derived(
    isNearToken
      ? ({
          decimals: 24,
          symbol: 'NEAR',
          name: 'NEAR',
          icon: NEAR_ICON,
          spec: '',
          reference: null,
          reference_hash: null
        } satisfies FtMetadata)
      : tokenId
        ? metadataMap.get(tokenId)
        : undefined
  );
  let currentBalance = $derived(tokenId ? balances.get(tokenId) : undefined);
  let currentDecimals = $derived(currentMeta?.decimals ?? 0);
  let displayBalance = $derived(
    currentBalance ? formatBalance(currentBalance, currentDecimals) : undefined
  );

  let tokenEntries = $derived(
    tokens.map(id => {
      const isNear = isWrapNear(id);
      return {
        id,
        metadata: isNear
          ? ({
              decimals: 24,
              symbol: 'NEAR',
              name: 'NEAR',
              icon: NEAR_ICON,
              spec: '',
              reference: null,
              reference_hash: null
            } satisfies FtMetadata)
          : metadataMap.get(id),
        balance: balances.get(id),
        loading: !metadataMap.has(id) && !isNear
      };
    })
  );

  let _insufficientBalance = $derived(
    !!(
      tokenId &&
      currentBalance &&
      rawAmount &&
      currentDecimals > 0 &&
      Number(amount) > 0
    ) &&
      (() => {
        let available = BigInt(currentBalance);
        if (isWrapNear(tokenId)) {
          available = available > NEAR_RESERVE ? available - NEAR_RESERVE : 0n;
        }
        return BigInt(rawAmount) > available;
      })()
  );

  $effect(() => {
    if (amount && tokenId && currentDecimals > 0) {
      const fn = toFixedNumber(amount, currentDecimals);
      rawAmount = fn?.toU128() ?? '';
    } else {
      rawAmount = amount;
    }
  });

  $effect(() => {
    insufficientBalance = _insufficientBalance;
  });

  $effect(() => {
    tokenSymbol = currentMeta?.symbol ?? '';
  });

  onMount(async () => {
    try {
      const { accountStore } = await import('$lib/near/account');
      accountStore.subscribe(v => {
        accountId = v;
      });
    } catch {}

    try {
      tokens = await contract.getTokenWhitelist();
      if (tokens.length > 0 && !tokenId) tokenId = tokens[0];
    } catch {
      tokens = [];
    }

    loading = false;

    loadMetadataAndBalances();
  });

  async function loadMetadataAndBalances() {
    if (tokens.length === 0) return;

    const nonWrap = tokens.filter(id => !isWrapNear(id));
    if (nonWrap.length > 0) {
      metadataMap = await fetchAllMetadata(nonWrap);
    } else {
      metadataMap = new Map();
    }

    if (accountId) {
      const balMap = new Map<string, string>();
      const balPromises: Promise<void>[] = [];

      for (const id of tokens) {
        if (isWrapNear(id)) {
          balPromises.push(
            getCombinedNearBalance(WRAP_NEAR_ID, accountId!)
              .then(b => {
                if (b) balMap.set(WRAP_NEAR_ID, b);
              })
              .catch(() => {})
          );
        } else {
          balPromises.push(
            getTokenBalance(id, accountId!)
              .then(b => {
                balMap.set(id, b);
              })
              .catch(() => {})
          );
        }
      }

      await Promise.all(balPromises);
      balances = balMap;
    }
  }

  function handleInput(e: Event) {
    const input = e.target as HTMLInputElement;
    let val = filterAllowedCharacters(input.value);

    if (val === '.') {
      val = '0.';
    }

    if (currentDecimals > 0) {
      const dotPos = val.indexOf('.');
      if (dotPos >= 0 && val.length - dotPos - 1 > currentDecimals) {
        val = val.slice(0, dotPos + 1 + currentDecimals);
      }
    }

    input.value = val;
    amount = val;
  }

  function handleBlur() {
    if (!amount) return;
    const formatted = getFormattedNumber(amount, currentDecimals);
    if (formatted) {
      amount = formatted;
      if (inputEl) inputEl.value = amount;
    }
  }

  function setMax() {
    if (!currentBalance || !currentMeta) return;
    let raw = BigInt(currentBalance);
    if (isWrapNear(tokenId)) {
      raw = raw > NEAR_RESERVE ? raw - NEAR_RESERVE : 0n;
    }
    const formatted = formatBalance(raw.toString(), currentDecimals).replace(
      /,/g,
      ''
    );
    amount = formatted;
    if (inputEl) inputEl.value = formatted;
  }

  function selectToken(id: string) {
    tokenId = id;
    amount = '';
    rawAmount = '';
  }
</script>

<div class="space-y-2">
  <button
    type="button"
    class="w-full flex items-center gap-2.5 bg-transparent border border-white/15 rounded px-3 py-2 text-sm hover:border-white/30 transition-colors"
    onclick={() => (modalOpen = true)}
    disabled={loading || tokens.length === 0 || disabled}
  >
    <img
      src={currentMeta?.icon ?? NEAR_ICON}
      alt={currentMeta?.symbol ?? '?'}
      class="w-5 h-5 rounded-full shrink-0"
    />

    <span class="flex-1 text-left truncate">
      {currentMeta?.symbol ?? (tokenId || 'Select token')}
    </span>

    {#if displayBalance != null}
      <span class="text-white/40 text-xs tabular-nums">
        {displayBalance}
      </span>
    {/if}

    <span class="text-white/30 text-xs">▼</span>
  </button>

  <div class="flex gap-2">
    <input
      bind:this={inputEl}
      type="text"
      value={amount}
      oninput={handleInput}
      onblur={handleBlur}
      placeholder="0.0"
      class="flex-1 bg-transparent border border-white/15 rounded px-3 py-2 text-sm focus:outline-none focus:border-primary tabular-nums"
      disabled={!tokenId || disabled}
      autocomplete="off"
    />
    {#if currentBalance}
      <button
        type="button"
        class="bg-white/5 border border-white/15 rounded px-3 py-2 text-xs text-white/60 hover:text-white/80 hover:bg-white/10 transition-colors shrink-0"
        onclick={setMax}
        {disabled}
      >
        MAX
      </button>
    {/if}
  </div>

  {#if amount && usdValue}
    <p class="text-xs text-white/40 text-right tabular-nums">{usdValue}</p>
  {/if}
  {#if _insufficientBalance}
    <p class="text-xs text-red-400 text-right">Insufficient balance</p>
  {/if}
</div>

<TokenSelectModal
  bind:open={modalOpen}
  tokens={tokenEntries}
  selectedId={tokenId}
  onselect={selectToken}
  onclose={() => (modalOpen = false)}
/>
