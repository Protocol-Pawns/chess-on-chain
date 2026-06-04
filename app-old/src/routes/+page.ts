export async function load() {
  const url = new URL(window.location.href);
  const loadedGameId = url.searchParams.get("game_id");
  const loadedTab = url.searchParams.get("tab") as "watch" | "play" | undefined;
  if (loadedTab == null && url.pathname === "/") {
    url.searchParams.set("tab", "play");
    location.href = url.toString();
  }

  return {
    loadedGameId: loadedGameId
      ? JSON.parse(decodeURI(loadedGameId))
      : undefined,
    loadedTab,
  };
}
