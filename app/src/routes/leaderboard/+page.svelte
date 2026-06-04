<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type AccountStats } from '$lib/api/client';
	import { contract } from '$lib/near/connector';

	interface LeaderEntry {
		account_id: string;
		elo: number;
		stats: AccountStats | null;
	}

	let entries = $state<LeaderEntry[]>([]);
	let loading = $state(true);
	let loadingMore = $state(false);
	let skip = $state(0);
	const PAGE_SIZE = 50;
	let hasMore = $state(true);

	async function loadPage(offset: number): Promise<{ items: LeaderEntry[]; hasMore: boolean }> {
		const eloRatings = await contract.getEloRatings(offset, PAGE_SIZE);
		if (eloRatings.length === 0) return { items: [], hasMore: false };

		const ids = eloRatings.map(([id]) => id);
		const statsMap = new Map<string, AccountStats>();
		try {
			const statsResults = await Promise.all(
				ids.map(id => api.accountStats(id).then(s => [id, s] as const).catch(() => null))
			);
			for (const r of statsResults) {
				if (r) statsMap.set(r[0], r[1]);
			}
		} catch { /* non-critical */ }

		const items: LeaderEntry[] = eloRatings.map(([account_id, elo]) => ({
			account_id,
			elo,
			stats: statsMap.get(account_id) ?? null
		}));

		return { items, hasMore: eloRatings.length === PAGE_SIZE };
	}

	async function load() {
		try {
			const res = await loadPage(0);
			entries = res.items;
			hasMore = res.hasMore;
			skip = entries.length;
		} catch (e) {
			console.error('Failed to load leaderboard:', e);
		} finally {
			loading = false;
		}
	}

	async function loadMore() {
		if (loadingMore || !hasMore) return;
		loadingMore = true;
		try {
			const res = await loadPage(skip);
			entries = [...entries, ...res.items];
			hasMore = res.hasMore;
			skip += res.items.length;
		} catch (e) {
			console.error('Failed to load more:', e);
		} finally {
			loadingMore = false;
		}
	}

	onMount(load);
</script>

<div class="flex flex-col gap-4">
	<h2 class="text-xl font-bold text-primary text-center">Leaderboard</h2>

	{#if loading}
		<div class="space-y-1.5 animate-pulse">
			{#each Array(10) as _}
				<div class="card flex items-center gap-3 py-2">
					<div class="h-4 w-6 rounded bg-white/10"></div>
					<div class="h-4 w-28 rounded bg-white/10 flex-1"></div>
					<div class="h-4 w-10 rounded bg-white/5"></div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="card">
			<table class="w-full text-sm">
				<thead>
					<tr class="text-white/50 text-xs">
						<th class="pb-2 text-left">#</th>
						<th class="pb-2 text-left">Player</th>
						<th class="pb-2 text-right">ELO</th>
						<th class="pb-2 text-right">W</th>
						<th class="pb-2 text-right">L</th>
						<th class="pb-2 text-right">D</th>
						<th class="pb-2 text-right">Rate</th>
					</tr>
				</thead>
				<tbody>
					{#each entries as entry, i}
						<tr class="border-t border-primary/20">
							<td class="py-1.5 text-white/40">{i + 1}</td>
							<td class="py-1.5">
								<a href="/profile/{entry.account_id}" class="text-primary hover:underline text-xs">{entry.account_id}</a>
							</td>
							<td class="py-1.5 text-right text-primary-warn font-semibold">{entry.elo}</td>
							{#if entry.stats}
								<td class="py-1.5 text-right text-primary-green">{entry.stats.wins}</td>
								<td class="py-1.5 text-right text-primary-err">{entry.stats.losses}</td>
								<td class="py-1.5 text-right text-white/50">{entry.stats.draws}</td>
								<td class="py-1.5 text-right text-white/70">{entry.stats.total_games > 0 ? ((entry.stats.wins / entry.stats.total_games) * 100).toFixed(1) : 0}%</td>
							{:else}
								<td class="py-1.5 text-right text-white/30">-</td>
								<td class="py-1.5 text-right text-white/30">-</td>
								<td class="py-1.5 text-right text-white/30">-</td>
								<td class="py-1.5 text-right text-white/30">-</td>
							{/if}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if hasMore}
			<button class="btn-secondary text-sm w-full" onclick={loadMore} disabled={loadingMore}>
				{loadingMore ? 'Loading...' : 'Load More'}
			</button>
		{/if}

		{#if entries.length === 0}
			<p class="text-white/50 text-sm text-center">No players yet. Be the first!</p>
		{/if}
	{/if}
</div>
