// @vitest-environment jsdom
import {afterEach,expect,it,vi} from "vitest";
vi.mock("./App",()=>({default:()=>null}));
vi.mock("./data/preview",()=>({books:[]}));
import {readSelection,type Workspace} from "./native";
afterEach(()=>{window.getSelection()?.removeAllRanges();document.body.replaceChildren();});
function fixture(){
  const state={session:{translation:"kjv"},books:[{id:1,name:"Genesis"}],passage:[1,2,3,4].map(n=>({id:`v${n}`,refs:[1001000+n],text:`Verse ${n} text`}))} as unknown as Workspace;
  const root=document.createElement("div");root.className="verse-list";
  root.innerHTML=state.passage.map((v,i)=>`<div class="verse-row"><button>${i+1}</button><p data-reference="Genesis 1:${i+1}"><span data-verse-text><span>Verse ${i+1}</span> text</span></p></div>`).join("");
  document.body.append(root);
  const verses=Array.from(root.querySelectorAll('[data-verse-text]'));
  return {state,root,verses};
}
function choose(start:Node,startOffset:number,end:Node,endOffset:number){
  const range=document.createRange();range.setStart(start,startOffset);range.setEnd(end,endOffset);
  window.getSelection()!.removeAllRanges();window.getSelection()!.addRange(range);
}
it("clips a multi-verse selection to its actual text boundaries",()=>{
  const {state,verses}=fixture();
  choose(verses[1].firstChild!.firstChild!,3,verses[2].lastChild!,3);
  expect(readSelection(state)?.nativeSelection.spans).toEqual([
    {verse_id:"v2",start:3,end:12,quote:"se 2 text"},
    {verse_id:"v3",start:0,end:10,quote:"Verse 3 te"},
  ]);
});
it("does not include the verse above an element-boundary selection",()=>{
  const {state,root}=fixture();
  choose(root,2,root,4);
  expect(readSelection(state)?.nativeSelection.spans.map(s=>s.verse_id)).toEqual(["v3","v4"]);
});
it("ignores a verse touched only at its final text boundary",()=>{
  const {state,verses}=fixture();
  choose(verses[1].lastChild!,5,verses[2].lastChild!,5);
  expect(readSelection(state)?.nativeSelection.spans.map(s=>s.verse_id)).toEqual(["v3"]);
});
