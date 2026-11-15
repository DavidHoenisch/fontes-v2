import { invoke, isTauri } from "@tauri-apps/api/core";

// Presentation only. Browsers and desktop builds have no vibration fallback.
export async function arrivalHaptic(): Promise<void> {
  if (!isTauri() || document.visibilityState === "hidden") return;
  try {
    await invoke("verse_arrival_haptic");
  } catch {
    // A missing/disabled haptic must never interrupt reading or navigation.
  }
}
