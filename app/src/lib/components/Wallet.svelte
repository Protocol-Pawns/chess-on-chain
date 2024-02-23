<script lang="ts">
  import type { Account } from "@near-wallet-selector/core";
  import { FixedNumber } from "@tarnadas/fixed-number";

  import { MessageBox } from "$lib/components";
  import { Section } from "$lib/layout";
  import type { Condition } from "$lib/models";

  export let account: Account;

  let condition: Condition;
  $: if (!account.accountId.endsWith(".tg")) {
    condition = "err";
  } else if (nearBalance != null && nearBalance.toNumber() < 1) {
    condition = "warn";
  } else {
    condition = "ok";
  }

  let nearBalance: FixedNumber | undefined;

  fetchNearBalance();

  async function fetchNearBalance() {
    const res = await fetch(import.meta.env.VITE_NODE_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: "dontcare",
        method: "query",
        params: {
          request_type: "view_account",
          finality: "final",
          account_id: account.accountId,
        },
      }),
    });
    const json = (await res.json()) as {
      result: { amount: string; locked: string };
    };
    if (!json.result) return;
    nearBalance = new FixedNumber(json.result.amount, 24).sub(
      new FixedNumber(json.result.locked, 24),
    );
  }
</script>

<Section header="Wallet" {condition}>
  <div class="field">
      You successfully connected your Near wallet {account.accountId}
  </div>
  <div class="field">
    <span>Near balance:</span>
    <span>{nearBalance ? nearBalance.format() : "-"}</span>
    {#if nearBalance != null && nearBalance.toNumber() < 0.5}
      <MessageBox type="warning">
        Your Near balance is low! Please top up your Near balance to not run out
        of gas.
      </MessageBox>
    {/if}
  </div>
</Section>
