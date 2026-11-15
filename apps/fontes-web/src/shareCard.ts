import { invoke, isTauri } from "@tauri-apps/api/core";

export type ShareQuote = { text: string; reference: string; source: string };
export type CardFormat = "portrait" | "square";
export const cardBackgrounds = [
  {id:"harbor",name:"Quiet harbor",url:"/art/share/quiet-harbor.jpg"},
  {id:"hills",name:"Olive hills",url:"/art/share/olive-hills.jpg"},
] as const;
export const cardSize = (format:CardFormat) => ({width:1080,height:format === "portrait" ? 1350 : 1080});

// Wrap without truncation, including long unbroken words and explicit line breaks.
export function wrapCardText(text:string, measure:(text:string)=>number, width:number):string[] {
  return text.split(/\r?\n/).flatMap(paragraph=>{
    const lines:string[]=[]; let line="";
    for(const word of paragraph.split(/\s+/).filter(Boolean)) {
      if(measure(line ? `${line} ${word}` : word) <= width) {line=line ? `${line} ${word}` : word;continue;}
      if(line) {lines.push(line);line="";}
      for(const char of Array.from(word)) {
        if(line && measure(line+char)>width) {lines.push(line);line="";}
        line+=char;
      }
    }
    lines.push(line);return lines;
  });
}

export async function renderShareCard(quote:ShareQuote, background:string, format:CardFormat):Promise<HTMLCanvasElement> {
  if(!quote.text.trim() || !quote.reference.trim() || !quote.source.trim()) throw Error("A quotation, reference and source are required.");
  await Promise.all([document.fonts.load('48px Lora'),document.fonts.load('24px Plex'),document.fonts.load('italic 28px Literary')]);
  const image = new Image(); image.src=background; await image.decode();
  const canvas=document.createElement("canvas"),size=cardSize(format);
  canvas.width=size.width;canvas.height=size.height;
  const ctx=canvas.getContext("2d");if(!ctx) throw Error("Image export is unavailable on this device.");
  const scale=Math.max(size.width/image.naturalWidth,size.height/image.naturalHeight);
  const w=image.naturalWidth*scale,h=image.naturalHeight*scale;
  ctx.drawImage(image,(size.width-w)/2,(size.height-h)/2,w,h);
  ctx.fillStyle="rgba(15,27,25,.48)";ctx.fillRect(0,0,size.width,size.height);
  const shade=ctx.createLinearGradient(0,0,0,size.height);
  shade.addColorStop(0,"rgba(13,23,22,.25)");shade.addColorStop(.75,"rgba(13,23,22,.1)");shade.addColorStop(1,"rgba(13,23,22,.65)");
  ctx.fillStyle=shade;ctx.fillRect(0,0,size.width,size.height);
  const margin=88,width=size.width-margin*2;
  ctx.textBaseline="top";ctx.fillStyle="#fff9ec";
  ctx.font="24px Plex";
  const source=wrapCardText(quote.source,t=>ctx.measureText(t).width,width);
  ctx.font="28px Plex";
  const reference=wrapCardText(quote.reference,t=>ctx.measureText(t).width,width);
  const attributionHeight=reference.length*36+source.length*32+24;
  const available=size.height-310-attributionHeight;
  let fontSize=66,lines:string[]=[];
  for(;fontSize>=28;fontSize-=2) {
    ctx.font=`${fontSize}px Lora`;
    lines=wrapCardText(`“${quote.text}”`,t=>ctx.measureText(t).width,width);
    if(lines.length*fontSize*1.4<=available) break;
  }
  if(fontSize<28) throw Error("This selection is too long for one card. Choose a shorter passage or try Portrait.");
  let y=140+Math.max(0,(available-lines.length*fontSize*1.4)/2);
  for(const line of lines) {ctx.fillText(line,margin,y);y+=fontSize*1.4;}
  y=size.height-110-attributionHeight;
  ctx.fillStyle="rgba(255,249,236,.6)";ctx.fillRect(margin,y-32,64,2);
  ctx.fillStyle="#fff9ec";ctx.font="28px Plex";
  for(const line of reference) {ctx.fillText(line,margin,y);y+=36;}
  ctx.font="24px Plex";ctx.fillStyle="rgba(255,249,236,.82)";y+=12;
  for(const line of source) {ctx.fillText(line,margin,y);y+=32;}
  ctx.font="italic 28px Literary";ctx.fillStyle="rgba(255,249,236,.8)";ctx.fillText("Fontes",margin,size.height-56);
  return canvas;
}
export function cardFile(canvas:HTMLCanvasElement):Promise<File> {
  return new Promise((resolve,reject)=>canvas.toBlob(blob=>blob ? resolve(new File([blob],"fontes-scripture.png",{type:"image/png"})) : reject(Error("Could not create the image.")),"image/png"));
}
export async function shareCard(file:File,dataUrl:string,quote:ShareQuote):Promise<void> {
  const caption=`“${quote.text}”\n— ${quote.reference}\n${quote.source}\nShared with Fontes`;
  if(isTauri()) {await invoke("share_verse_image",{dataUrl,caption});return;}
  if(navigator.canShare?.({files:[file]})) {await navigator.share({files:[file],title:quote.reference,text:caption});return;}
  downloadCard(file);
}
export function downloadCard(file:File) {
  const url=URL.createObjectURL(file),anchor=document.createElement("a");
  anchor.href=url;anchor.download=file.name;anchor.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
}
