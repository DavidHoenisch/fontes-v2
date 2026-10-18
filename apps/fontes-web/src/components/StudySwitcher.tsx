import { useLayoutEffect, useRef, useState } from "react";
const sections = [["words", "Words"], ["commentary", "Commentary"], ["topic", "Topics"], ["dictionary", "Dictionaries"]] as const;
export function StudySwitcher({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const row = useRef<HTMLDivElement>(null);
  const [indicator, setIndicator] = useState({left:0,width:0});
  const active = Math.max(0, sections.findIndex(([id]) => id === value));
  useLayoutEffect(() => {
    const update = () => {
      const button = buttons.current[active];
      if (button) setIndicator({left:button.offsetLeft,width:button.offsetWidth});
    };
    update();
    const observer = new ResizeObserver(update);
    if (row.current) observer.observe(row.current);
    buttons.current.forEach(button => { if (button) observer.observe(button); });
    const button = buttons.current[active], scroll = row.current?.parentElement;
    if (button && scroll) scroll.scrollTo({left:Math.max(0,button.offsetLeft-(scroll.clientWidth-button.offsetWidth)/2),behavior:matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth"});
    return () => observer.disconnect();
  }, [active]);
  return <div className="study-switcher-scroll">
    <div ref={row} className="study-switcher" role="group" aria-label="Study sections">
      {sections.map(([id, label], index) => <button
        key={id} ref={el => { buttons.current[index] = el; }} type="button"
        aria-pressed={value === id} onClick={() => onChange(id)}
        onKeyDown={event => {
          const next = event.key === "ArrowRight" ? (index + 1) % sections.length : event.key === "ArrowLeft" ? (index + sections.length - 1) % sections.length : event.key === "Home" ? 0 : event.key === "End" ? sections.length - 1 : null;
          if (next !== null) { event.preventDefault(); buttons.current[next]?.focus(); onChange(sections[next][0]); }
        }}
      >{label}</button>)}
      <span className="study-switcher-line" aria-hidden="true" style={{ transform: `translateX(${indicator.left}px)`, width:indicator.width }} />
    </div>
  </div>;
}
