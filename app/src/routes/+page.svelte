<script lang="ts">
  import Tab, { Label } from "@smui/tab";
  import TabBar from "@smui/tab-bar";
  import { writable, type Writable } from "svelte/store";

  import { Play, Watch } from "./_index";

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

  const loadedUrl = new URL(window.location.href);
  const loadedTab = loadedUrl.searchParams.get("tab");
  let active$: Writable<(typeof tabs)[0]>;
  if (loadedTab === "watch") {
    active$ = writable(tabs[1]);
  } else {
    active$ = writable(tabs[0]);
  }

  active$.subscribe((active) => {
    const url = new URL(window.location.href);
    url.searchParams.set("tab", active.label.toLowerCase());
    window.history.replaceState({}, "", url);
  });
</script>

<div class="page">
  <TabBar {tabs} let:tab bind:active={$active$}>
    <Tab {tab}>
      <Label>{tab.label}</Label>
    </Tab>
  </TabBar>

  <svelte:component this={$active$.component} />
</div>

<style lang="scss">
  .page {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
</style>
