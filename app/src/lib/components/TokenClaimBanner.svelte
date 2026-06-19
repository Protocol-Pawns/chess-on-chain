<script lang="ts">
  import { escrowTokens, refreshEscrowTokens } from '$lib/near/balances';
  import { contract } from '$lib/near/connector';
  import { showTxToast } from '$lib/toast';
  import TokenBalanceList from '$lib/components/TokenBalanceList.svelte';

  let withdrawing = $state<string | null>(null);

  function handleWithdraw(tokenId: string) {
    withdrawing = tokenId;
    showTxToast(
      contract.withdrawToken(tokenId).finally(() => {
        withdrawing = null;
        setTimeout(refreshEscrowTokens, 4000);
      })
    );
  }
</script>

{#if $escrowTokens.length > 0}
  <div
    class="rounded-lg border-2 border-primary-warn/60 bg-primary-warn/10 p-3 space-y-2"
  >
    <div class="flex items-center gap-2">
      <svg
        viewBox="0 0 24 24"
        class="w-5 h-5 text-primary-warn shrink-0"
        fill="currentColor"
      >
        <path
          d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"
        />
      </svg>
      <h3 class="text-sm font-bold text-primary-warn">
        You have {$escrowTokens.length}
        {$escrowTokens.length === 1 ? 'token' : 'tokens'} to claim!
      </h3>
    </div>
    <TokenBalanceList
      tokens={$escrowTokens}
      showWithdraw
      onWithdraw={handleWithdraw}
      {withdrawing}
    />
  </div>
{/if}
