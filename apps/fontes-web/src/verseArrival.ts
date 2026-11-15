export const VERSE_ARRIVAL_MS = 1600;
const activeArrivals = new WeakMap<HTMLElement, () => void>();

// Replacing or cancelling an arrival restores the original text and annotations.
export function arriveAtVerse(verse: HTMLElement, onArrival?: () => void | Promise<void>) {
  const list = verse.parentElement;
  if (!list || !verse.isConnected) return () => {};
  activeArrivals.get(list)?.();
  void list.offsetWidth;
  verse.classList.add("verse-arrival");
  const clear = () => {
    clearTimeout(timer);
    if (activeArrivals.get(list) !== clear) return;
    verse.classList.remove("verse-arrival");
    activeArrivals.delete(list);
  };
  const timer = setTimeout(clear, VERSE_ARRIVAL_MS);
  activeArrivals.set(list, clear);
  try {
    void Promise.resolve(onArrival?.()).catch(() => {});
  } catch {
    // Optional tactile feedback is independent of the visual cue.
  }
  return clear;
}
