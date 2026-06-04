<script lang="ts">
  import Button from "@smui/button";
  import { parseNearAmount } from "near-api-js/lib/utils/format";

  import { MessageBox } from "$lib/components";
  import ProgressSpinner from "$lib/components/ProgressSpinner.svelte";
  import { contract$, wallet, type ProtocolPawnsContract } from "$lib/near";

  let isRegistered: boolean | undefined;

  const accountId$ = wallet.accountId$;
  $: if ($contract$ && $accountId$ != null) {
    checkIsRegistered($contract$, $accountId$);
  }

  async function checkIsRegistered(
    c: Promise<ProtocolPawnsContract>,
    accountId: string,
  ) {
    const contract = await c;

    const bal = await contract.storage_balance_of({
      account_id: accountId,
    });
    isRegistered = bal != null;
  }

  async function register() {
    await wallet.signAndSendTransaction(
      {
        receiverId: import.meta.env.VITE_CONTRACT_ID!,
        actions: [
          {
            type: "FunctionCall",
            params: {
              methodName: "storage_deposit",
              args: {
                registration_only: true,
              },
              gas: "30000000000000",
              deposit: parseNearAmount("0.05") ?? "0",
            },
          },
        ],
      },
      {},
    );
  }
</script>

<div class="play">
  <h3>Welcome to Protocol Pawns!</h3>
  <div>
    Protocol Pawns is the very first fully decentralized on-chain chess game
    built on Near Protocol. Challenge other wallets to play against you or play
    against an AI. Earn points by playing and winning. Complete recurring quests
    and collect achievements!
  </div>
  <div>
    Learn more about the game in the <a href="./about">about section</a>.
  </div>

  {#if $accountId$ == null}
    <MessageBox type="info">
      Please login in order to play chess via Protocol Pawns!
    </MessageBox>
  {:else if isRegistered == null}
    <ProgressSpinner inline />
  {:else if isRegistered}
    TODO
  {:else}
    <div>
      In order to play you first need to register your account. This will cost a
      small fee of 0.05N in order for the contract to pay for the used storage.
    </div>
    <Button variant="outlined" on:click={register}>Register</Button>
  {/if}
</div>

<style lang="scss">
  .play {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  h3 {
    margin-bottom: 0.4rem;
    align-self: center;
  }
</style>
