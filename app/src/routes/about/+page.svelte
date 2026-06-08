<script lang="ts">
  import { onMount } from 'svelte';
  import {
    accountStore,
    isLoggedIn,
    isRegistered,
    register
  } from '$lib/near/account';
  import { contract } from '$lib/near/connector';
  import { showTxToast } from '$lib/toast';
  import { fmtPPP, fmtOneDecimal } from '$lib/format';
  import PppIcon from '$lib/components/PppIcon.svelte';

  let questList = $state<
    Array<{
      name: string;
      points: string;
      points_on_cd: string;
      cooldown: number;
    }>
  >([]);
  let achievementList = $state<Array<{ name: string; points: string }>>([]);
  let loading = $state(true);

  function formatCooldown(ms: number): string {
    const hours = Math.round(ms / 3600000);
    if (hours >= 24) {
      return `~${fmtOneDecimal(hours / 24)}d`;
    }
    return `~${hours}h`;
  }

  const QUEST_LABELS: Record<string, string> = {
    DailyPlayMove: 'Play a Move',
    WeeklyWinHuman: 'Win vs Human'
  };

  const ACHIEVEMENT_LABELS: Record<string, string> = {
    FirstWinHuman: 'First Win vs Human',
    FirstWinAiEasy: 'First Win vs AI (Easy)',
    FirstWinAiMedium: 'First Win vs AI (Medium)',
    FirstWinAiHard: 'First Win vs AI (Hard)'
  };

  onMount(async () => {
    try {
      const [q, a] = await Promise.all([
        contract.getQuestList().catch(() => []),
        contract.getAchievementList().catch(() => [])
      ]);
      questList = q;
      achievementList = a;
    } catch (e) {
      console.error('Failed to load contract data:', e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="flex flex-col gap-4">
  <h2 class="text-xl font-bold text-primary text-center">
    About Protocol Pawns
  </h2>

  <section class="flex flex-col gap-2">
    <h3 class="text-base font-semibold">What is Protocol Pawns?</h3>
    <p class="text-sm text-white/80 leading-relaxed">
      Protocol Pawns is the very first fully decentralized on-chain chess game
      built on
      <a
        href="https://near.org"
        target="_blank"
        rel="noopener"
        class="text-primary hover:underline">NEAR Protocol</a
      >. Every move is recorded on-chain, every game is verifiable, and there
      are no centralized servers controlling your games.
    </p>
  </section>

  <section class="flex flex-col gap-2">
    <h3 class="text-base font-semibold">How to Play</h3>
    <div class="flex flex-col gap-1 text-sm text-white/80">
      <div class="card">
        <div class="font-semibold text-primary mb-1">1. Connect & Register</div>
        <p>
          Connect your NEAR wallet and register with a one-time storage deposit
          of 0.05 N.
        </p>
      </div>
      <div class="card">
        <div class="font-semibold text-primary mb-1">2. Start a Game</div>
        <p>
          Challenge another player by entering their wallet address, or start a
          game against the AI with three difficulty levels:
        </p>
        <ul class="list-disc list-inside mt-1 text-white/60">
          <li>
            <span class="text-primary-green">Easy</span> — casual play, low gas (~8
            TGas)
          </li>
          <li>
            <span class="text-primary-warn">Medium</span> — balanced challenge (~30
            TGas)
          </li>
          <li>
            <span class="text-primary-err">Hard</span> — strong opponent (~110 TGas)
          </li>
        </ul>
      </div>
      <div class="card">
        <div class="font-semibold text-primary mb-1">3. Make Your Moves</div>
        <p>
          Click or drag pieces on the board. Each move is a signed on-chain
          transaction. The contract validates all moves so you can't cheat.
        </p>
      </div>
      <div class="card">
        <div class="font-semibold text-primary mb-1">4. Win & Earn</div>
        <p>
          Earn PPP (Protocol Pawns Points) for playing and winning. Complete
          daily quests and unlock achievements. Your ELO rating is tracked
          on-chain.
        </p>
      </div>
    </div>
  </section>

  <section class="flex flex-col gap-2">
    <h3 class="text-base font-semibold">
      <PppIcon size={22} /> Points System (PPP)
    </h3>
    <p class="text-sm text-white/80 leading-relaxed">
      PPP (Protocol Pawns Points) is a non-transferable on-chain token. You earn
      it by playing — every move counts, every win matters. Points are awarded
      through <span class="text-primary">quests</span> (repeatable) and
      <span class="text-primary">achievements</span> (one-time).
    </p>

    {#if loading}
      <div class="card animate-pulse">
        <div class="h-4 w-32 rounded bg-white/10 mb-2"></div>
        <div class="space-y-1">
          {#each Array(2) as _}
            <div class="h-6 rounded bg-white/5"></div>
          {/each}
        </div>
      </div>
    {:else if questList.length > 0}
      <div class="card">
        <div class="font-semibold text-primary mb-2">Quests (Repeatable)</div>
        <table class="w-full text-sm">
          <thead>
            <tr class="text-white/50 text-left text-xs">
              <th class="pb-1">Quest</th>
              <th class="pb-1 text-right">Full Points</th>
              <th class="pb-1 text-right">On Cooldown</th>
              <th class="pb-1 text-right">Cooldown</th>
            </tr>
          </thead>
          <tbody>
            {#each questList as q}
              <tr class="border-t border-white/10">
                <td class="py-1.5">{QUEST_LABELS[q.name] ?? q.name}</td>
                <td class="py-1.5 text-right text-primary-green"
                  >{fmtPPP(q.points)}</td
                >
                <td class="py-1.5 text-right text-white/50"
                  >{fmtPPP(q.points_on_cd)}</td
                >
                <td class="py-1.5 text-right text-white/50"
                  >{formatCooldown(q.cooldown)}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="text-xs text-white/40 mt-1">
          First completion earns full points. Subsequent completions during
          cooldown earn diminished points.
        </p>
      </div>
    {/if}

    {#if !loading && achievementList.length > 0}
      <div class="card">
        <div class="font-semibold text-primary mb-2">
          Achievements (One-Time)
        </div>
        <table class="w-full text-sm">
          <thead>
            <tr class="text-white/50 text-left text-xs">
              <th class="pb-1">Achievement</th>
              <th class="pb-1 text-right">Points</th>
            </tr>
          </thead>
          <tbody>
            {#each achievementList as a}
              <tr class="border-t border-white/10">
                <td class="py-1.5">{ACHIEVEMENT_LABELS[a.name] ?? a.name}</td>
                <td class="py-1.5 text-right text-primary-green"
                  >{fmtPPP(a.points)}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="text-xs text-white/40 mt-1">
          Each achievement can only be earned once. Only victories count —
          losses and draws award 0 points.
        </p>
      </div>
    {/if}
  </section>

  <section class="flex flex-col gap-2">
    <h3 class="text-base font-semibold">ELO Rating</h3>
    <p class="text-sm text-white/80 leading-relaxed">
      Every game you finish updates your ELO rating, stored directly on the
      smart contract. Win against stronger opponents to climb faster. Your
      rating is visible on your profile page and on the <a
        href="/leaderboard"
        class="text-primary hover:underline">leaderboard</a
      >.
    </p>
  </section>

  <section class="flex flex-col gap-2">
    <h3 class="text-base font-semibold">Smart Contract</h3>
    <p class="text-sm text-white/80">
      The chess engine runs entirely inside the NEAR smart contract at
      <a
        href="https://explorer.near.org/accounts/app.chess-game.near"
        target="_blank"
        rel="noopener"
        class="text-primary hover:underline font-mono text-xs"
        >app.chess-game.near</a
      >. It validates legal moves, detects checkmate and stalemate, and emits
      NEP-297 events for every game action.
    </p>
  </section>

  <section class="flex flex-col gap-2">
    <h3 class="text-base font-semibold">Tech Stack</h3>
    <ul class="text-sm text-white/80 space-y-1">
      <li class="flex gap-2">
        <span class="text-primary">-</span>
        <span class="font-semibold">Contract:</span> Rust on NEAR Protocol
      </li>
      <li class="flex gap-2">
        <span class="text-primary">-</span>
        <span class="font-semibold">Indexer:</span> GoldSky (NEP-297 event pipeline)
      </li>
      <li class="flex gap-2">
        <span class="text-primary">-</span>
        <span class="font-semibold">API:</span> Cloudflare Workers + Hyperdrive (PostgreSQL)
      </li>
      <li class="flex gap-2">
        <span class="text-primary">-</span>
        <span class="font-semibold">Frontend:</span> SvelteKit + Svelte 5 + UnoCSS
      </li>
      <li class="flex gap-2">
        <span class="text-primary">-</span>
        <span class="font-semibold">Wallet:</span> @hot-labs/near-connect
      </li>
    </ul>
  </section>

  {#if $isLoggedIn && !$isRegistered}
    <section class="card text-sm flex flex-col gap-2">
      <p>Ready to play? Register now with a one-time 0.05 N storage deposit.</p>
      <button
        class="btn-primary text-sm self-start"
        onclick={() => showTxToast(register())}
      >
        Register Now
      </button>
    </section>
  {/if}
</div>
