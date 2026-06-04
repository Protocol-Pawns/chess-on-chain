<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api, type AccountStats, type GameOverview } from '$lib/api/client';
	import { contract } from '$lib/near/connector';
	import GameCard from '$lib/components/GameCard.svelte';

	const accountId = page.params.id ?? '';

	let stats = $state<AccountStats | null>(null);
	let games = $state<GameOverview[]>([]);
	let elo = $state<number | null>(null);
	let points = $state<string | null>(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			const [s, accountData] = await Promise.all([
				api.accountStats(accountId),
				contract.getAccount(accountId).catch(() => null)
			]);
			stats = s;
			if (accountData) {
				elo = accountData.elo;
				points = accountData.points;
			}

			const account = await api.account(accountId);
			if (account.finishedGameIds.length > 0) {
				const res = await fetch('https://api.protocol-pawns.com/query', {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ gameIds: account.finishedGameIds })
				});
				if (res.ok) games = await res.json();
			}
		} catch (e) {
			console.error('Failed to load profile:', e);
		} finally {
			loading = false;
		}
	});
</script>

{#if loading}
	<div class="text-center py-12 text-white/50">Loading...</div>
{:else}
	<div class="space-y-6">
		<section class="card">
			<h2 class="text-lg font-bold mb-1 text-primary">{accountId}</h2>

			{#if elo !== null || points !== null}
				<div class="grid {elo !== null && points !== null ? 'grid-cols-2' : 'grid-cols-1'} gap-3 mb-3">
					{#if elo !== null}
						<div class="text-center bg-primary-transparent2 rounded p-2">
							<div class="text-xl font-bold text-primary-warn">{elo}</div>
							<div class="text-xs text-white/50">ELO</div>
						</div>
					{/if}
					{#if points !== null}
						<div class="text-center bg-primary-transparent2 rounded p-2">
							<div class="text-xl font-bold text-primary">{points}</div>
							<div class="text-xs text-white/50">Points</div>
						</div>
					{/if}
				</div>
			{/if}

			{#if stats}
				<div class="grid grid-cols-4 gap-3">
					<div class="text-center">
						<div class="text-lg font-bold text-primary-green">{stats.wins}</div>
						<div class="text-xs text-white/50">Wins</div>
					</div>
					<div class="text-center">
						<div class="text-lg font-bold text-primary-err">{stats.losses}</div>
						<div class="text-xs text-white/50">Losses</div>
					</div>
					<div class="text-center">
						<div class="text-lg font-bold">{stats.draws}</div>
						<div class="text-xs text-white/50">Draws</div>
					</div>
					<div class="text-center">
						<div class="text-lg font-bold text-primary">
							{stats.total_games > 0 ? ((stats.wins / stats.total_games) * 100).toFixed(1) : 0}%
						</div>
						<div class="text-xs text-white/50">Win Rate</div>
					</div>
				</div>
			{/if}
		</section>

		<section>
			<h3 class="text-base font-semibold mb-2">Finished Games</h3>
			{#if games.length === 0}
				<p class="text-white/50 text-sm">No games yet</p>
			{:else}
				<div class="space-y-2">
					{#each games as game}
						<a href="/game/{encodeURIComponent(JSON.stringify(game.game_id))}">
							<GameCard {game} />
						</a>
					{/each}
				</div>
			{/if}
		</section>
	</div>
{/if}
