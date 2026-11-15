import { useState } from "react";
import { Button } from "@heroui/react";
import { ArrowUpRight, ChevronRight } from "lucide-react";
export type OccurrenceReference = { ref_id: number; label: string };
export type OccurrencePage = { verses: OccurrenceReference[]; has_more: boolean };
export function OccurrenceExplorer({books,loadPage,loadVerse,onRead}: {
  books:{id:number;name:string;verses:number}[];
  loadPage:(book:number,offset:number)=>Promise<OccurrencePage>;
  loadVerse:(reference:number)=>Promise<string>;
  onRead:(reference:number)=>Promise<void>;
}) {
  const [all,setAll] = useState(false);
  return <div className="occurrence-explorer">
    <p className="muted small">Browse occurrences by book. Expand a verse to read it here, or open it in Read.</p>
    {(all ? books : books.slice(0,5)).map(book => <OccurrenceBook key={book.id} book={book} loadPage={loadPage} loadVerse={loadVerse} onRead={onRead}/>)}
    {books.length > 5 && <Button variant="ghost" size="sm" className="mt-3" onPress={()=>setAll(!all)}>{all ? "Show fewer books" : `Show all ${books.length} books`}</Button>}
  </div>;
}
function OccurrenceBook({book,loadPage,loadVerse,onRead}: {
  book:{id:number;name:string;verses:number};
  loadPage:(book:number,offset:number)=>Promise<OccurrencePage>;
  loadVerse:(reference:number)=>Promise<string>;
  onRead:(reference:number)=>Promise<void>;
}) {
  const [open,setOpen] = useState(false),[rows,setRows] = useState<OccurrenceReference[]>([]),[loaded,setLoaded] = useState(false),[more,setMore] = useState(false),[busy,setBusy] = useState(false),[error,setError] = useState("");
  async function load() {
    if(busy) return;
    setBusy(true);setError("");
    try { const page=await loadPage(book.id,rows.length); setRows(previous=>[...previous,...page.verses.filter(row=>!previous.some(v=>v.ref_id===row.ref_id))]);setMore(page.has_more);setLoaded(true); }
    catch(e) {setError(String(e));} finally {setBusy(false);}
  }
  return <div className="occurrence-book-section">
    <button type="button" className="occurrence-disclosure occurrence-book-toggle" aria-expanded={open} onClick={()=>{setOpen(!open);if(!open && !loaded) void load();}}>
      <ChevronRight size={17} aria-hidden="true"/><span>{book.name}</span>{" "}<small>{book.verses.toLocaleString()} verses</small>
    </button>
    {open && <div className="occurrence-reference-list">
      {rows.map(row=><OccurrenceVerse key={row.ref_id} reference={row} loadVerse={loadVerse} onRead={onRead}/>)}
      {busy && <p role="status" className="muted small">Loading references…</p>}
      {error && <div role="alert"><p>{error}</p><Button variant="ghost" size="sm" onPress={()=>void load()}>Retry references</Button></div>}
      {loaded && !rows.length && !busy && <p className="muted small">No reference previews in this library sample.</p>}
      {more && !error && <Button variant="ghost" size="sm" isDisabled={busy} onPress={()=>void load()}>Show more references in {book.name}</Button>}
    </div>}
  </div>;
}
function OccurrenceVerse({reference,loadVerse,onRead}: {reference:OccurrenceReference;loadVerse:(ref:number)=>Promise<string>;onRead:(ref:number)=>Promise<void>}) {
  const [open,setOpen]=useState(false),[text,setText]=useState<string|null>(null),[loading,setLoading]=useState(false),[previewError,setPreviewError]=useState("");
  const [busy,setBusy]=useState(false),[error,setError]=useState("");
  async function load() {
    if(loading) return;
    setLoading(true);setPreviewError("");
    try {setText(await loadVerse(reference.ref_id));}catch(e){setPreviewError(String(e));}finally{setLoading(false);}
  }
  async function navigate() {
    if(busy) return;
    setBusy(true);setError("");
    try {await onRead(reference.ref_id);}catch(e){setError(String(e));}finally{setBusy(false);}
  }
  return <div className="occurrence-reference">
    <button type="button" className="occurrence-disclosure" aria-expanded={open} onClick={()=>{setOpen(!open);if(!open && text===null) void load();}}>
      <ChevronRight size={15} aria-hidden="true"/><span>{reference.label}</span>
    </button>
    {open && <div className="occurrence-verse-text">
      {loading && <p role="status" className="muted small">Loading verse…</p>}
      {previewError && <div role="alert"><p>{previewError}</p><Button variant="ghost" size="sm" onPress={()=>void load()}>Retry verse</Button></div>}
      {text !== null && <p className="scripture">{text || "Verse text is not available in this edition."}</p>}
      <Button variant="secondary" size="sm" aria-label={`Open ${reference.label} in Read`} isDisabled={busy} onPress={()=>void navigate()}>
        <ArrowUpRight size={16} aria-hidden="true"/>{busy ? "Opening…" : "Open in Read"}
      </Button>
      {error && <div role="alert"><p>{error}</p><Button variant="ghost" size="sm" onPress={()=>void navigate()}>Retry opening verse</Button></div>}
    </div>}
  </div>;
}
