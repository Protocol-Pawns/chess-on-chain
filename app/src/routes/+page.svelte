<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type GlobalStats, type GameOverview, type LeaderboardEntry } from '$lib/api/client';
	import { contract } from '$lib/near/connector';
	import { isLoggedIn } from '$lib/near/account';
	import { showTxToast } from '$lib/toast';
	import GameCard from '$lib/components/GameCard.svelte';

	let stats = $state<GlobalStats | null>(null);
	let activeGames = $state<GameOverview[]>([]);
	let finishedGames = $state<GameOverview[]>([]);
	let loading = $state(true);
	let showAiMenu = $state(false);

	onMount(async () => {
		try {
			const [s, ag, fg] = await Promise.all([
				api.stats(),
				api.games('active', undefined, 20),
				api.games('finished', undefined, 20)
			]);
			stats = s;
			activeGames = ag.items;
			finishedGames = fg.items;
		} catch (e) {
			console.error('Failed to load lobby data:', e);
		} finally {
			loading = false;
		}
	});

	function createAiGame(difficulty: 'Easy' | 'Medium' | 'Hard') {
		showAiMenu = false;
		showTxToast(contract.createAiGame(difficulty));
	}
</script>

{#if loading}
	<div class="text-center py-12 text-white/50">Loading...</div>
{:else}
	<section class="mb-6">
		<div class="flex items-center justify-between mb-4">
			<div></div>
			{#if $isLoggedIn}
				<div class="flex gap-2">
					<a href="/challenges" class="btn-secondary text-sm">Challenge</a>
					<div class="relative">
						<button class="btn-primary text-sm" onclick={() => showAiMenu = !showAiMenu}>
							vs AI
						</button>
						{#if showAiMenu}
							<div class="absolute right-0 top-full mt-1 card min-w-32 z-50 space-y-1">
								<button class="btn-secondary w-full text-left text-sm" onclick={() => createAiGame('Easy')}>Easy</button>
								<button class="btn-secondary w-full text-left text-sm" onclick={() => createAiGame('Medium')}>Medium</button>
								<button class="btn-secondary w-full text-left text-sm" onclick={() => createAiGame('Hard')}>Hard</button>
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</div>
		{#if stats}
			<div class="grid grid-cols-2 gap-3">
				<div class="card text-center">
					<div class="text-xl font-bold text-primary">{stats.total_games}</div>
					<div class="text-xs text-white/50">Total Games</div>
				</div>
				<div class="card text-center">
					<div class="text-xl font-bold text-primary-green">{stats.active_games}</div>
					<div class="text-xs text-white/50">Active</div>
				</div>
				<div class="card text-center">
					<div class="text-xl font-bold">{stats.finished_games}</div>
					<div class="text-xs text-white/50">Finished</div>
				</div>
				<div class="card text-center">
					<div class="text-xl font-bold">{stats.total_moves}</div>
					<div class="text-xs text-white/50">Total Moves</div>
				</div>
			</div>
		{/if}
	</section>

	<section class="space-y-8">
		<div>
			<h3 class="text-base font-semibold mb-2">Active Games</h3>
			{#if activeGames.length === 0}
				<p class="text-white/50 text-sm">No active games right now</p>
			{:else}
				<div class="space-y-2">
					{#each activeGames as game}
						<a href="/game/{encodeURIComponent(JSON.stringify(game.game_id))}">
							<GameCard {game} />
						</a>
					{/each}
				</div>
			{/if}
		</div>

		<div>
			<h3 class="text-base font-semibold mb-2">Recent Games</h3>
			{#if finishedGames.length === 0}
				<p class="text-white/50 text-sm">No finished games yet</p>
			{:else}
				<div class="space-y-2">
					{#each finishedGames as game}
						<a href="/game/{encodeURIComponent(JSON.stringify(game.game_id))}">
							<GameCard {game} />
						</a>
					{/each}
				</div>
			{/if}
		</div>
	</section>
{/if}
