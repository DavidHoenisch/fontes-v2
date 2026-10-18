// One transient spotlight per chapter; replacing it restores every previous row.
export function spotlightVerse(verse: HTMLElement) {
  const list = verse.parentElement;
  if (!list) return () => {};
  list.classList.remove("spotlight-active");
  list.querySelectorAll(".verse-arrival").forEach(v => v.classList.remove("verse-arrival"));
  void list.offsetWidth;
  verse.classList.add("verse-arrival");
  list.classList.add("spotlight-active");
  const clear = () => {
    clearTimeout(timer);
    list.classList.remove("spotlight-active");
    verse.classList.remove("verse-arrival");
  };
  const timer = setTimeout(clear, 2400);
  return clear;
}
