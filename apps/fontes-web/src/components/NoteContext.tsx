import { useEffect, useRef, useState } from "react";
import { refId, type Live } from "../native";

export function noteDate(value: string, now = new Date()) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const day = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(day); yesterday.setDate(day.getDate() - 1);
  if (date >= day && date <= now) return "Today";
  if (date >= yesterday && date < day) return "Yesterday";
  return new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", ...(date.getFullYear() !== now.getFullYear() ? { year: "numeric" } : {}) }).format(date);
}

// The selection is authoritative; otherwise show passage context without changing the note.
export function NoteContext({ quote, anchor, live }: { quote: string; anchor: string; live?: Live }) {
  const element = useRef<HTMLDivElement>(null);
  const [passage, setPassage] = useState("");
  const ref = live ? refId(live.state, anchor) : 0;
  const loaded = live?.state.passage.find(v => v.refs.includes(ref))?.text;
  useEffect(() => {
    setPassage("");
    if (quote || loaded || !live) return;
    let active = true;
    const load = () => {
      void live.send({ command: "note.passage", params: { ref_id: ref } }).then(r => {
        if (active) setPassage((r.data.block as { quote: string }).quote);
      }).catch(() => { /* Missing editions leave the canonical reference visible. */ });
    };
    if (typeof IntersectionObserver === "undefined") { load(); return () => { active = false; }; }
    const observer = new IntersectionObserver(entries => {
      if (entries.some(e => e.isIntersecting)) { observer.disconnect(); load(); }
    }, { rootMargin: "100px" });
    if (element.current) observer.observe(element.current);
    return () => { active = false; observer.disconnect(); };
  }, [quote, loaded, ref, live?.state.content_id, live?.state.session.translation]);
  const text = quote || loaded || passage;
  return <div ref={element} className={text ? "note-quote" : "note-context-pending"}>{text}</div>;
}
