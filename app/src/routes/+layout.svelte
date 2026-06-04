<script lang="ts">
	import WalletButton from '$lib/components/WalletButton.svelte';
	import Toasts from '$lib/components/Toasts.svelte';
	import 'virtual:uno.css';
	import '@unocss/reset/tailwind.css';

	let { children } = $props();
</script>

<div class="min-h-screen flex flex-col bg-bg text-white">
	<header class="flex items-center justify-end flex-wrap gap-4 px-3 py-2 min-h-14 bg-primary-transparent">
		<a href="/" class="flex-1 flex items-center justify-center gap-2">
			<img src="/favicon.png" alt="logo" class="h-8" />
			<h1 class="text-lg font-bold text-primary">Protocol Pawns</h1>
		</a>
		<WalletButton />
	</header>

	<main class="flex-1 overflow-y-auto flex justify-center items-start px-3 py-5">
		<div class="w-full" style="max-width: min(100%, 30rem);">
			{@render children()}
		</div>
	</main>

	<footer class="w-full px-3 py-2">
		<div class="flex items-center gap-2">
			<a href="https://near.org/" target="_blank" rel="noopener" class="text-white/60 hover:text-primary text-xs">
				Built on NEAR
			</a>
			<a href="https://github.com/Protocol-Pawns" target="_blank" rel="noopener" class="text-white/60 hover:text-primary text-xs">
				GitHub
			</a>
			<a href="https://twitter.com/ProtocolPawns" target="_blank" rel="noopener" class="text-white/60 hover:text-primary text-xs">
				Twitter
			</a>
			<a href="https://t.me/protocolpawns" target="_blank" rel="noopener" class="text-white/60 hover:text-primary text-xs">
				Telegram
			</a>
		</div>
	</footer>
	<Toasts />
</div>
