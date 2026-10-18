import { useRef, useState } from "react";
import { Button } from "@heroui/react";
import type { Live } from "../native";
export type Citation = { start_ref: number | null; end_ref: number | null; basis: string };
export function ResourceCitations({live, links}: {live: Live; links: Citation[]}) {
  const [selected, setSelected] = useState("");
  const [verses, setVerses] = useState<{ref_id: number; label: string; text: string}[]>([]);
  const [loading, setLoading] = useState(false), [error, setError] = useState("");
  const request = useRef(0);
  const label = (ref: number) => `${live.state.books.find(b => b.id === Math.floor(ref / 1000000))?.name || ""} ${Math.floor(ref % 1000000 / 1000)}:${ref % 1000}`;
  const targets = links.filter((l, i, all) => l.start_ref && l.end_ref && all.findIndex(a => a.start_ref === l.start_ref && a.end_ref === l.end_ref) === i);
  if (!targets.length) return null;
  async function open(link: Citation) {
    const token = ++request.current;
    setSelected(label(link.start_ref!) + (link.end_ref !== link.start_ref ? ` – ${label(link.end_ref!)}` : ""));
    setLoading(true); setError(""); setVerses([]);
    try {
      const r = await live.send({command:"reading.preview", params:{start_ref:link.start_ref, end_ref:link.end_ref}});
      if (token === request.current) setVerses(r.data.verses as typeof verses);
    } catch(e) {if (token === request.current) setError(String(e));}
    finally {if (token === request.current) setLoading(false);}
  }
  return <div className="resource-citations">
    <h3>Scripture references</h3>
    <p className="muted small">Read a cited passage here without leaving the article.</p>
    <div className="flex flex-wrap gap-2 mt-4">{targets.map(l => <Button key={`${l.start_ref}-${l.end_ref}`} size="sm" variant="tertiary" onPress={() => void open(l)}>{label(l.start_ref!)}{l.end_ref !== l.start_ref ? ` – ${label(l.end_ref!)}` : ""}</Button>)}</div>
    {selected && <div className="attachment-preview mt-4"><div className="card-meta">{selected}</div>
      {loading && <p role="status">Loading passage…</p>}{error && <p role="alert">{error}</p>}
      {verses.map(v => <p className="scripture" key={v.ref_id}><small>{v.label}</small><br/>{v.text}</p>)}
    </div>}
  </div>;
}
