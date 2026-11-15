import {useEffect,useRef,useState,type ReactNode} from "react";
import {createReadingSelectionGuard} from "../readingSelectionGuard";
import {readSelection,type NativeSelection,type Workspace} from "../native";
export type ReaderSelection={quote:string;anchor:string;endRef?:number;nativeSelection?:NativeSelection};

// Handle drags update only this sibling control surface, never the reader tree.
export function ReadingSelection({state,seed,children}:{state?:Workspace;seed:ReaderSelection|null;children:(selection:ReaderSelection,snapshot:()=>ReaderSelection,dismiss:()=>void)=>ReactNode}) {
  const [current,setCurrent]=useState(seed);
  const latest=useRef(seed);
  const readCurrent=useRef<()=>ReaderSelection|null>(()=>null);
  useEffect(()=>{latest.current=seed;setCurrent(seed);},[seed]);
  useEffect(()=>{
    if(!state)return;
    let timer:ReturnType<typeof setTimeout>;
    const guard=createReadingSelectionGuard();
    const capture=()=>{
      const root=document.querySelector<HTMLElement>(".verse-list");
      const selection=window.getSelection();
      if(!root || !selection || !guard.accept(root,selection))return null;
      return readSelection(state);
    };
    readCurrent.current=capture;
    const changed=()=>{
      const selected=capture();
      if(!selected)return;
      latest.current=selected;
      clearTimeout(timer);
      timer=setTimeout(()=>setCurrent(latest.current),160);
    };
    changed();
    document.addEventListener("selectionchange",changed);
    return()=>{readCurrent.current=()=>null;clearTimeout(timer);document.removeEventListener("selectionchange",changed);};
  },[state]);
  if(!current)return null;
  return <>{children(current,()=> readCurrent.current()||latest.current||current,()=>{latest.current=null;setCurrent(null);})}</>;
}
