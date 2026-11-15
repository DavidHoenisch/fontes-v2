// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { arriveAtVerse, VERSE_ARRIVAL_MS } from "./verseArrival";
afterEach(() => {vi.useRealTimers();document.body.innerHTML="";});
function chapter() {
  document.body.innerHTML = '<div class="verse-list"><div class="verse-row"><p class="verse"><mark>Saved highlight</mark></p></div><div class="verse-row"><p class="verse">Second verse</p></div></div>';
  return document.body.firstElementChild as HTMLElement;
}
it("cues only the target, restores annotations, and replaces the previous arrival", () => {
  vi.useFakeTimers();
  const list=chapter(), first=list.children[0] as HTMLElement, second=list.children[1] as HTMLElement;
  const haptic=vi.fn();
  const clear=arriveAtVerse(first,haptic);
  expect(list.className).toBe("verse-list");
  expect(first.classList.contains("verse-arrival")).toBe(true);
  expect(second.className).toBe("verse-row");
  vi.advanceTimersByTime(300);
  arriveAtVerse(second,haptic);
  clear(); // A stale cleanup cannot cancel the replacement.
  expect(first.classList.contains("verse-arrival")).toBe(false);
  expect(second.classList.contains("verse-arrival")).toBe(true);
  vi.advanceTimersByTime(VERSE_ARRIVAL_MS-1);
  expect(second.classList.contains("verse-arrival")).toBe(true);
  vi.advanceTimersByTime(1);
  expect(second.className).toBe("verse-row");
  expect(first.querySelector("mark")?.textContent).toBe("Saved highlight");
  expect(haptic).toHaveBeenCalledTimes(2);
});
it("keeps the destination cue beyond the old brief animation and then restores it", () => {
  vi.useFakeTimers();
  const row=chapter().firstElementChild as HTMLElement;
  arriveAtVerse(row);
  vi.advanceTimersByTime(1000);
  expect(row.classList.contains("verse-arrival")).toBe(true);
  vi.advanceTimersByTime(VERSE_ARRIVAL_MS-1000);
  expect(row.className).toBe("verse-row");
});
it("does not pulse or animate a detached destination", () => {
  const pulse=vi.fn(), row=document.createElement("div");
  arriveAtVerse(row,pulse)();
  expect(pulse).not.toHaveBeenCalled();
  expect(row.className).toBe("");
});
it("restarts the same verse without an older cleanup removing the new cue", () => {
  vi.useFakeTimers();
  const row=chapter().firstElementChild as HTMLElement;
  const old=arriveAtVerse(row);
  vi.advanceTimersByTime(100);
  arriveAtVerse(row);
  old();
  expect(row.classList.contains("verse-arrival")).toBe(true);
  vi.advanceTimersByTime(VERSE_ARRIVAL_MS);
  expect(row.className).toBe("verse-row");
});
it("clears on cancellation and keeps the visual cue when haptics fail", async () => {
  vi.useFakeTimers();
  const row=chapter().firstElementChild as HTMLElement;
  const clear=arriveAtVerse(row,() => {throw Error("No haptics");});
  expect(row.classList.contains("verse-arrival")).toBe(true);
  clear();
  expect(row.className).toBe("verse-row");
  arriveAtVerse(row,async () => {throw Error("Unavailable");});
  await Promise.resolve();
  vi.advanceTimersByTime(VERSE_ARRIVAL_MS);
  expect(row.className).toBe("verse-row");
});
