<script lang="ts">
  interface Props {
    page: number;
    totalPages: number;
    onchange: (page: number) => void;
  }

  let { page, totalPages, onchange }: Props = $props();

  function visiblePages(current: number, total: number): number[] {
    if (total <= 7) {
      return Array.from({ length: total }, (_, i) => i + 1);
    }
    const pages: Set<number> = new Set([1, total, current]);
    for (let i = current - 1; i <= current + 1; i++) {
      if (i >= 1 && i <= total) pages.add(i);
    }
    return Array.from(pages).sort((a, b) => a - b);
  }

  function hasGapBefore(p: number, pages: number[]): boolean {
    const idx = pages.indexOf(p);
    return idx > 0 && pages[idx - 1] !== p - 1;
  }

  function hasGapAfter(p: number, pages: number[]): boolean {
    const idx = pages.indexOf(p);
    return idx < pages.length - 1 && pages[idx + 1] !== p + 1;
  }
</script>

{#if totalPages > 1}
  <div class="flex items-center justify-center gap-1 text-sm">
    <button
      class="btn text-xs px-4"
      onclick={() => onchange(page - 1)}
      disabled={page <= 1}
    >
      &lt;
    </button>

    {#each visiblePages(page, totalPages) as p (p)}
      {#if hasGapBefore(p, visiblePages(page, totalPages))}
        <span class="text-white/30 px-1 select-none">&hellip;</span>
      {/if}
      <button
        class="text-xs px-2 py-1 rounded transition-colors {p === page
          ? 'bg-primary text-white font-semibold'
          : 'text-white/50 hover:text-white hover:bg-white/10'}"
        onclick={() => onchange(p)}
        disabled={p === page}
      >
        {p}
      </button>
      {#if hasGapAfter(p, visiblePages(page, totalPages))}
        <span class="text-white/30 px-1 select-none">&hellip;</span>
      {/if}
    {/each}

    <button
      class="btn text-xs px-4"
      onclick={() => onchange(page + 1)}
      disabled={page >= totalPages}
    >
      &gt;
    </button>
  </div>
{/if}
