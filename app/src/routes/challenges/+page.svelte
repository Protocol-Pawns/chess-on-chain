<script lang="ts">
	import { onMount } from 'svelte';
	import { api, type Challenge } from '$lib/api/client';
	import { accountStore, isLoggedIn } from '$lib/near/account';
	import { contract } from '$lib/near/connector';
	import { showTxToast } from '$lib/toast';

	let challenges = $state<Challenge[]>([]);
	let loading = $state(true);
	let challengeTarget = $state('');

	async function load() {
		if (!$accountStore) return;
		try {
			challenges = await api.challenges($accountStore);
		} catch (e) {
			console.error('Failed to load challenges:', e);
		} finally {
			loading = false;
		}
	}

	function sendChallenge() {
		if (!$accountStore || !challengeTarget.trim()) return;
		const target = challengeTarget.trim();
		challengeTarget = '';
		showTxToast(contract.challenge(target));
		setTimeout(load, 4000);
	}

	function acceptChallenge(id: string) {
		showTxToast(contract.acceptChallenge(id));
		setTimeout(load, 4000);
	}

	function rejectChallenge(id: string) {
		showTxToast(contract.rejectChallenge(id));
		setTimeout(load, 4000);
	}

	onMount(load);
</script>

{#if !$isLoggedIn}
	<div class="text-center py-12 text-white/50">Connect your wallet to view challenges</div>
{:else if loading}
	<div class="text-center py-12 text-white/50">Loading...</div>
{:else}
	<div class="space-y-6">
		<section class="card">
			<h2 class="text-base font-semibold mb-2">Challenge a Player</h2>
			<div class="flex gap-2">
				<input
					type="text"
					bind:value={challengeTarget}
					placeholder="account.near"
					class="flex-1 bg-transparent border border-primary rounded px-2 py-1.5 text-sm focus:outline-none focus:border-primary-light"
				/>
				<button class="btn-primary text-sm" onclick={sendChallenge} disabled={!challengeTarget.trim()}>
					Challenge
				</button>
			</div>
		</section>

		<section>
			<h2 class="text-base font-semibold mb-2">Your Challenges</h2>
			{#if challenges.length === 0}
				<p class="text-white/50 text-sm">No challenges yet</p>
			{:else}
				<div class="space-y-2">
					{#each challenges as challenge}
						<div class="card flex items-center justify-between">
							<div>
								<div class="font-medium text-sm">
									{challenge.challenger === $accountStore ? '→' : '←'}
									{challenge.challenger === $accountStore ? challenge.challenged : challenge.challenger}
								</div>
								<div class="text-xs text-white/50">
									{challenge.status}
								</div>
							</div>
							<div class="flex gap-2">
								{#if challenge.status === 'pending' && challenge.challenged === $accountStore}
									<button class="btn-primary text-xs" onclick={() => acceptChallenge(challenge.id)}>Accept</button>
									<button class="btn-secondary text-xs" onclick={() => rejectChallenge(challenge.id)}>Reject</button>
								{/if}
								{#if challenge.status === 'accepted' && challenge.game_id}
									<a href="/game/{encodeURIComponent(challenge.game_id)}" class="btn-primary text-xs">View Game</a>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</section>
	</div>
{/if}
