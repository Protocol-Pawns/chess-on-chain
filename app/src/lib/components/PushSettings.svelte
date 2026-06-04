<script lang="ts">
	import { pushEnabled, enablePush, disablePush, isLoggedIn } from '$lib/near/account';
	import { showTxToast } from '$lib/toast';

	let loading = $state(false);

	async function handleToggle() {
		loading = true;
		try {
			if ($pushEnabled) {
				await disablePush();
			} else {
				await enablePush();
			}
		} catch (e) {
			console.error('Push toggle failed:', e);
		} finally {
			loading = false;
		}
	}
</script>

{#if $isLoggedIn}
	<div class="card">
		<div class="flex items-start gap-3">
			<div class="text-2xl mt-0.5">🔔</div>
			<div class="flex-1">
				<h3 class="text-sm font-semibold mb-1">Push Notifications</h3>
				{#if $pushEnabled}
					<p class="text-xs text-white/60 mb-2">You'll be notified when it's your turn, when a challenge arrives, or when a game ends.</p>
					<button class="btn-secondary text-xs" onclick={handleToggle} disabled={loading}>
						{loading ? '...' : 'Disable Notifications'}
					</button>
				{:else}
					<p class="text-xs text-white/60 mb-2">
						Never miss a move! Get instant browser notifications when:
					</p>
					<ul class="text-xs text-white/50 mb-2 space-y-0.5">
						<li>It's your turn in a game</li>
						<li>Someone challenges you</li>
						<li>A game finishes</li>
					</ul>
					<button class="btn-primary text-xs" onclick={handleToggle} disabled={loading}>
						{loading ? '...' : 'Enable Notifications'}
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}
