// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { spotlightVerse } from "./verseArrival";
afterEach(() => {vi.useRealTimers();document.body.innerHTML="";});
it("restores the chapter after the cue and cancels the previous arrival on replacement", () => {
  vi.useFakeTimers();
  document.body.innerHTML='<div class="verse-list"><div class="verse-row"><mark>Saved highlight</mark></div><div class="verse-row"></div></div>';
  const list=document.body.firstElementChild as HTMLElement, first=list.children[0] as HTMLElement, second=list.children[1] as HTMLElement;
  const clear=spotlightVerse(first);
  expect(list.classList.contains("spotlight-active")).toBe(true);
  vi.advanceTimersByTime(1000);clear();spotlightVerse(second);
  expect(first.classList.contains("verse-arrival")).toBe(false);
  expect(second.classList.contains("verse-arrival")).toBe(true);
  vi.advanceTimersByTime(1400);
  expect(list.classList.contains("spotlight-active")).toBe(true);
  vi.advanceTimersByTime(1000);
  expect(list.classList.contains("spotlight-active")).toBe(false);
  expect(second.classList.contains("verse-arrival")).toBe(false);
  expect(first.querySelector("mark")?.textContent).toBe("Saved highlight");
});
