import { useRef } from "react";
const sections = [["words", "Words"], ["commentary", "Commentary"], ["topic", "Topics"], ["dictionary", "Dictionaries"]] as const;
export function StudySwitcher({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const active = Math.max(0, sections.findIndex(([id]) => id === value));
  return <div className="study-switcher-scroll">
    <div className="study-switcher" role="group" aria-label="Study sections">
      {sections.map(([id, label], index) => <button
        key={id} ref={el => { buttons.current[index] = el; }} type="button"
        aria-pressed={value === id} onClick={() => onChange(id)}
        onKeyDown={event => {
          const next = event.key === "ArrowRight" ? (index + 1) % sections.length : event.key === "ArrowLeft" ? (index + sections.length - 1) % sections.length : event.key === "Home" ? 0 : event.key === "End" ? sections.length - 1 : null;
          if (next !== null) { event.preventDefault(); buttons.current[next]?.focus(); onChange(sections[next][0]); }
        }}
      >{label}</button>)}
      <span className="study-switcher-line" aria-hidden="true" style={{ transform: `translateX(${active * 100}%)` }} />
    </div>
  </div>;
}
