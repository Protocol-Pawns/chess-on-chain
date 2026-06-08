<script lang="ts">
  let { size = 16, class: cls = '' }: { size?: number; class?: string } =
    $props();
</script>

<img
  src="/icons/ppp.webp"
  alt="PPP"
  width={size}
  height={size}
  class="inline align-text-bottom {cls}"
/>
