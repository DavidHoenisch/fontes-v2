import { useEffect, useRef, useState } from "react";
import { Button } from "@heroui/react";
import { isTauri } from "@tauri-apps/api/core";
import { Share2 } from "lucide-react";
import { Sheet } from "./Sheet";
import { cardBackgrounds,cardFile,downloadCard,renderShareCard,shareCard,type CardFormat,type ShareQuote } from "../shareCard";
import "./share-card.css";

export function ShareCard({quote,onClose}:{quote:ShareQuote|null;onClose:()=>void}) {
  const [background,setBackground]=useState<string>(cardBackgrounds[0].url),[format,setFormat]=useState<CardFormat>("portrait");
  const [ready,setReady]=useState<{key:string;file:File;dataUrl:string}|null>(null),[error,setError]=useState(""),[busy,setBusy]=useState(false),[retry,setRetry]=useState(0);
  const preview=useRef<HTMLCanvasElement>(null);
  const key=JSON.stringify([quote,background,format,retry]);
  useEffect(()=>{
    let active=true;setReady(null);setError("");
    if(!quote) {setBusy(false);return;}
    void renderShareCard(quote,background,format).then(async canvas=>{
      const file=await cardFile(canvas);
      if(!active || !preview.current) return;
      preview.current.width=canvas.width;preview.current.height=canvas.height;
      preview.current.getContext("2d")?.drawImage(canvas,0,0);
      setReady({key,file,dataUrl:canvas.toDataURL("image/png")});
    }).catch(e=>{if(active)setError(String(e));});
    return()=>{active=false;};
  },[key]);
  const available=ready?.key===key;
  return <Sheet title="Share a passage" eyebrow="WORDS TO CARRY" description="A little beauty. The words and their source." open={quote!==null} onClose={onClose} className="share-card-sheet" footer={<><Button fullWidth isDisabled={!available||busy} onPress={async()=>{
    if(!quote||!ready||!available)return;setBusy(true);setError("");
    try{await shareCard(ready.file,ready.dataUrl,quote);}catch(e){if(!(e instanceof DOMException && e.name==="AbortError"))setError(String(e));}finally{setBusy(false);}
  }}><Share2 size={18}/>{busy?"Opening share options…":"Share image"}</Button>{!isTauri()&&<Button variant="ghost" fullWidth isDisabled={!available} onPress={()=>{if(ready&&available)downloadCard(ready.file);}}>Download PNG</Button>}</>}>
    <div className="share-card-preview" style={{aspectRatio:format==="portrait"?"4 / 5":"1"}}>
      <canvas ref={preview} role="img" aria-label={quote?`Share card: ${quote.text} — ${quote.reference}. ${quote.source}`:"Share card"} style={{visibility:available?"visible":"hidden"}}/>
      {!available&&!error&&<p role="status">Preparing your card…</p>}
    </div>
    <fieldset className="share-card-backgrounds"><legend>Choose a background</legend>{cardBackgrounds.map(b=><button key={b.id} type="button" aria-pressed={background===b.url} onClick={()=>setBackground(b.url)}><img src={b.url} alt=""/><span>{b.name}</span></button>)}</fieldset>
    <fieldset className="share-card-formats"><legend>Shape</legend>{([['portrait','Portrait · 4:5'],['square','Square · 1:1']] as const).map(([value,name])=><Button key={value} variant={format===value?"secondary":"ghost"} aria-pressed={format===value} onPress={()=>setFormat(value)}>{name}</Button>)}</fieldset>
    <p className="share-card-credit">Includes {quote?.reference} · {quote?.source}. Original generated artwork, inspired by painted landscapes.</p>
    {error&&<div role="alert"><p>{error}</p>{!available&&<Button variant="ghost" onPress={()=>setRetry(n=>n+1)}>Retry card</Button>}</div>}
  </Sheet>;
}
