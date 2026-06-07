<script lang="ts">
  import { page } from '$app/state';
  import { slide } from 'svelte/transition';
  import WalletButton from '$lib/components/WalletButton.svelte';
  import Toasts from '$lib/components/Toasts.svelte';
  import dayjs from 'dayjs';
  import localizedFormat from 'dayjs/plugin/localizedFormat';
  import 'virtual:uno.css';
  import '@unocss/reset/tailwind.css';

  dayjs.extend(localizedFormat);

  let { children } = $props();
  let showNav = $state(false);

  const links = [
    { href: '/', label: 'Home' },
    { href: '/leaderboard', label: 'Leaderboard' },
    { href: '/challenges', label: 'Challenges' },
    { href: '/bets', label: 'Bets' },
    { href: '/about', label: 'About' }
  ];

  function isActive(href: string): boolean {
    const path = page.url?.pathname ?? '/';
    if (href === '/') return path === '/';
    return path.startsWith(href);
  }

  function closeNav() {
    showNav = false;
  }
</script>

<div class="h-screen flex flex-col bg-bg text-white">
  <header
    class="shrink-0 flex items-center justify-end flex-wrap gap-4 px-3 py-2 min-h-14 bg-primary-transparent"
  >
    <div class="flex-1 flex items-center">
      <a href="/" class="inline-flex items-center gap-2" onclick={closeNav}>
        <img src="/favicon.png" alt="logo" class="h-8" />
        <h1 class="text-lg font-bold text-primary">Protocol Pawns</h1>
      </a>
    </div>

    <nav class="hidden sm:flex items-center gap-2">
      {#each links as link}
        <a
          href={link.href}
          class="btn text-sm {isActive(link.href)
            ? 'bg-primary-transparent2 border-primary-light text-primary-light'
            : 'hover:bg-primary-transparent'}"
        >
          {link.label}
        </a>
      {/each}
    </nav>

    <WalletButton />

    <button
      class="sm:hidden text-primary p-1 w-8 h-8 flex items-center justify-center"
      onclick={() => (showNav = !showNav)}
      aria-label="Menu"
    >
      <div class="w-6 h-5 relative flex flex-col justify-between">
        <span
          class="block h-0.5 bg-current rounded transition-all duration-200 origin-center {showNav
            ? 'translate-y-[9px] rotate-45'
            : ''}"
        ></span>
        <span
          class="block h-0.5 bg-current rounded transition-all duration-200 {showNav
            ? 'opacity-0'
            : 'opacity-100'}"
        ></span>
        <span
          class="block h-0.5 bg-current rounded transition-all duration-200 origin-center {showNav
            ? '-translate-y-[9px] -rotate-45'
            : ''}"
        ></span>
      </div>
    </button>

    {#if showNav}
      <nav
        class="w-full flex flex-col gap-1 items-center text-sm pb-1"
        transition:slide={{ duration: 200 }}
      >
        {#each links as link}
          <a
            href={link.href}
            class="btn w-60 text-center {isActive(link.href)
              ? 'bg-primary-transparent2'
              : 'hover:bg-primary-transparent'}"
            onclick={closeNav}
          >
            {link.label}
          </a>
        {/each}
      </nav>
    {/if}
  </header>

  <div class="flex-1 overflow-y-auto flex flex-col">
    <main class="flex-1 flex justify-center px-3 py-5">
      <div class="w-full" style="max-width: min(100%, 30rem);">
        {@render children()}
      </div>
    </main>

    <footer class="w-full px-4 py-3">
      <div class="flex items-center justify-center gap-4">
        <a
          href="https://near.org/"
          target="_blank"
          rel="noopener"
          class="text-white/40 hover:text-primary transition-colors"
          title="Built on NEAR"
        >
          <svg viewBox="0 0 16 16" class="w-4 h-4" fill="currentColor"
            ><path
              d="M13.9017 0.646973C13.3462 0.646973 12.8304 0.929357 12.5394 1.39357L9.40403 5.95741C9.30186 6.10784 9.34331 6.31059 9.49675 6.41075C9.6211 6.49202 9.78561 6.48196 9.89889 6.38645L12.9851 3.76201C13.0364 3.71677 13.1154 3.72137 13.1616 3.77165C13.1825 3.79469 13.1937 3.82444 13.1937 3.85502V12.0719C13.1937 12.1397 13.1377 12.1942 13.0684 12.1942C13.0312 12.1942 12.9962 12.1783 12.9727 12.1502L3.6435 1.20169C3.33966 0.850174 2.89352 0.647392 2.42388 0.646973H2.09782C1.21536 0.646973 0.5 1.34833 0.5 2.2135V13.7863C0.5 14.6515 1.21536 15.3528 2.09782 15.3528C2.65336 15.3528 3.16915 15.0704 3.46017 14.6062L6.59558 10.0424C6.69769 9.89194 6.65624 9.68919 6.50279 9.58903C6.37845 9.50776 6.21394 9.51782 6.10071 9.61334L3.01446 12.2378C2.96318 12.283 2.88412 12.2784 2.83797 12.2281C2.81703 12.2051 2.80592 12.1753 2.80634 12.1447V3.92583C2.80634 3.85796 2.86232 3.80349 2.93155 3.80349C2.96831 3.80349 3.00377 3.81941 3.02728 3.84748L12.3552 14.7981C12.659 15.1496 13.1052 15.3524 13.5749 15.3528H13.9009C14.7833 15.3532 15.4992 14.6523 15.5 13.7871V2.2135C15.5 1.34833 14.7842 0.646973 13.9017 0.646973Z"
            /></svg
          >
        </a>
        <a
          href="https://github.com/Protocol-Pawns"
          target="_blank"
          rel="noopener"
          class="text-white/40 hover:text-primary transition-colors"
          title="GitHub"
        >
          <svg viewBox="0 0 24 24" class="w-4 h-4" fill="currentColor"
            ><path
              d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.008-.866-.013-1.7-2.782.604-3.369-1.34-3.369-1.34-.454-1.156-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.831.092-.646.35-1.086.636-1.336-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0112 6.836c.85.004 1.705.114 2.504.336 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.203 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.578.688.48C19.138 20.163 22 16.418 22 12c0-5.523-4.477-10-10-10z"
            /></svg
          >
        </a>
        <a
          href="https://twitter.com/ProtocolPawns"
          target="_blank"
          rel="noopener"
          class="text-white/40 hover:text-primary transition-colors"
          title="Twitter / X"
        >
          <svg viewBox="0 0 24 24" class="w-4 h-4" fill="currentColor"
            ><path
              d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"
            /></svg
          >
        </a>
        <a
          href="https://t.me/protocolpawns"
          target="_blank"
          rel="noopener"
          class="text-white/40 hover:text-primary transition-colors"
          title="Telegram"
        >
          <svg viewBox="0 0 24 24" class="w-4 h-4" fill="currentColor"
            ><path
              d="M11.944 0A12 12 0 000 12a12 12 0 0012 12 12 12 0 0012-12A12 12 0 0012 0a12 12 0 00-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 01.171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.479.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"
            /></svg
          >
        </a>
      </div>
    </footer>
  </div>
  <Toasts />
</div>

<style>
  :global(*) {
    scrollbar-width: thin;
    scrollbar-color: #555 #222;
  }
  :global(*::-webkit-scrollbar) {
    width: 8px;
    height: 8px;
  }
  :global(*::-webkit-scrollbar-track) {
    background: #222;
  }
  :global(*::-webkit-scrollbar-thumb) {
    background: #555;
    border-radius: 4px;
  }
  :global(*::-webkit-scrollbar-thumb:hover) {
    background: #777;
  }
</style>
