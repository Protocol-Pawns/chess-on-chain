<script lang="ts">
  let {
    pieces = []
  }: {
    pieces?: string[];
  } = $props();

  const PIECE_IMG: Record<string, string> = {
    K: '/pieces/wK.webp',
    Q: '/pieces/wQ.webp',
    R: '/pieces/wR.webp',
    B: '/pieces/wB.webp',
    N: '/pieces/wN.webp',
    P: '/pieces/wP.webp',
    k: '/pieces/bK.webp',
    q: '/pieces/bQ.webp',
    r: '/pieces/bR.webp',
    b: '/pieces/bB.webp',
    n: '/pieces/bN.webp',
    p: '/pieces/bP.webp'
  };
</script>

<div class="flex items-center min-h-6 flex-wrap">
  {#each pieces as piece}
    <img
      src={PIECE_IMG[piece]}
      alt={piece}
      class="w-5 h-5 object-contain -ml-0.5"
      draggable="false"
    />
  {/each}
</div>
