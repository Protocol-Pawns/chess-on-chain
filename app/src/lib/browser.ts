export async function isBraveBrowser(): Promise<boolean> {
  try {
    const brave = (
      navigator as { brave?: { isBrave?: () => Promise<boolean> } }
    ).brave;
    if (!brave?.isBrave) return false;
    return await brave.isBrave();
  } catch {
    return false;
  }
}
