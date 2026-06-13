<script lang="ts">
  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let {
    open,
    onclose,
    children
  }: Props & { children?: import('svelte').Snippet } = $props();
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
    onclick={onclose}
    onkeydown={e => e.key === 'Escape' && onclose()}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="mx-4" onclick={e => e.stopPropagation()}>
      {@render children?.()}
    </div>
  </div>
{/if}
