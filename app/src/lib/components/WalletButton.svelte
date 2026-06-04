<script lang="ts">
	import { accountStore, connect, disconnect, isLoggedIn, isRegistered, isCheckingRegistration, register } from '$lib/near/account';
	import { showTxToast } from '$lib/toast';

	let showMenu = $state(false);
</script>

<div class="relative">
	{#if $isLoggedIn}
		<button class="btn-secondary text-sm" onclick={() => showMenu = !showMenu}>
			<span class="truncate max-w-24 sm:max-w-32">{$accountStore}</span>
		</button>
		{#if showMenu}
			<div class="absolute right-0 top-full mt-1 card min-w-40 z-50 space-y-1">
				<a href="/profile/{$accountStore}" class="block btn-secondary w-full text-left text-sm" onclick={() => showMenu = false}>
					Profile
				</a>
				{#if !$isRegistered && !$isCheckingRegistration}
					<button class="btn-primary w-full text-left text-sm" onclick={() => { showTxToast(register()); showMenu = false; }}>
						Register (0.05 N)
					</button>
				{/if}
				<button class="btn-secondary w-full text-left text-sm" onclick={() => { disconnect(); showMenu = false; }}>
					Disconnect
				</button>
			</div>
		{/if}
	{:else}
		<button class="btn-primary text-sm" onclick={connect}>
			Login
		</button>
	{/if}
</div>
