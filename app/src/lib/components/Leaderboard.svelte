<script lang="ts">
  import type { LeaderboardEntry } from '$lib/api/client';
  import { fmtOneDecimal } from '$lib/format';

  let { entries }: { entries: LeaderboardEntry[] } = $props();
</script>

<div class="card overflow-x-auto">
  <h3 class="text-base font-semibold mb-2">Leaderboard</h3>
  <table class="w-full text-sm">
    <thead>
      <tr class="text-white/50 text-left">
        <th class="pb-1">#</th>
        <th class="pb-1">Player</th>
        <th class="pb-1 text-right">W</th>
        <th class="pb-1 text-right">L</th>
        <th class="pb-1 text-right">D</th>
        <th class="pb-1 text-right">Rate</th>
      </tr>
    </thead>
    <tbody>
      {#each entries as entry, i}
        <tr class="border-t border-primary/30">
          <td class="py-1 text-white/50">{i + 1}</td>
          <td class="py-1">
            <a
              href="/profile/{entry.account_id}"
              class="text-primary hover:underline truncate max-w-24 inline-block"
              >{entry.account_id}</a
            >
          </td>
          <td class="py-1 text-right text-primary-green">{entry.wins}</td>
          <td class="py-1 text-right text-primary-err">{entry.losses}</td>
          <td class="py-1 text-right text-white/70">{entry.draws}</td>
          <td class="py-1 text-right">{fmtOneDecimal(entry.win_rate * 100)}%</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
