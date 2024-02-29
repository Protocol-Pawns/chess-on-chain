<script lang="ts">
  import Tab, { Label } from "@smui/tab";
  import TabBar from "@smui/tab-bar";
  import { writable, type Writable } from "svelte/store";

  import type { PageData } from "./$types";
  import { Play, Watch } from "./_index";

  import { pushState } from "$app/navigation";
  import { navigating } from "$app/stores";
  import { Game } from "$lib/components";
  import { gameId$ } from "$lib/game";

  export let data: PageData;
  if (data.loadedGameId) {
    $gameId$ = data.loadedGameId;
  }

  gameId$.subscribe((gameId) => {
    const url = new URL(window.location.href);
    if (!(url instanceof URL)) return;
    const oldUrl = url.toString();
    if (gameId) {
      url.searchParams.set("game_id", encodeURI(JSON.stringify(gameId)));
    } else {
      url.searchParams.delete("game_id");
    }
    if (oldUrl !== url.toString()) {
      pushState(url, {});
    }
  });

  let tabs = [
    {
      label: "Play",
      component: Play,
    },
    {
      label: "Watch",
      component: Watch,
    },
  ];

  let active$: Writable<(typeof tabs)[0]>;
  if (data.loadedTab === "watch") {
    active$ = writable(tabs[1]);
  } else {
    active$ = writable(tabs[0]);
  }

  active$.subscribe((active) => {
    const url = new URL(window.location.href);
    const oldUrl = url.toString();
    url.searchParams.set("tab", active.label.toLowerCase());
    if (oldUrl !== url.toString()) {
      pushState(url, {});
    }
  });

  navigating.subscribe(() => {
    const url = new URL(window.location.href);
    const loadedTab = url.searchParams.get("tab");
    if (loadedTab == null && url.pathname === "/") {
      url.searchParams.set("tab", $active$.label.toLowerCase());
      location.href = url.toString();
    }
  });
</script>

<div class="page">
  {#if $gameId$}
    <Game gameId={$gameId$} watchMode={$active$.label === "Watch"} />
  {:else}
    <TabBar {tabs} let:tab bind:active={$active$}>
      <Tab {tab}>
        <Label>{tab.label}</Label>
      </Tab>
    </TabBar>

    <svelte:component this={$active$.component} />
  {/if}
</div>

<style lang="scss">
  .page {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
</style>
