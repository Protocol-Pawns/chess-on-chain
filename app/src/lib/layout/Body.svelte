<div class="body">
  <slot />
</div>

<style lang="scss">
  .body {
    display: flex;
    justify-content: center;
    align-items: flex-start;
    min-width: 100vw;
    max-width: 100vw;
    overflow-y: auto;
    flex: 1 1 auto;
    padding: 1.8rem 0.4rem;

    :global(> *) {
      max-width: min(100%, 50rem);
    }
  }
</style>
