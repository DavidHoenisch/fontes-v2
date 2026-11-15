// @vitest-environment jsdom
import {afterEach,expect,it} from "vitest";
import {createReadingSelectionGuard} from "./readingSelectionGuard";
afterEach(()=>{window.getSelection()?.removeAllRanges();document.body.replaceChildren();});
function fixture(){
  document.body.innerHTML='<h1>Psalms 146</h1><div class="verse-list"><p>Verse one</p><p>Verse two</p><p>Verse three</p><p>Verse four</p><p>Verse five</p></div><footer>App navigation</footer>';
  const root=document.querySelector<HTMLElement>('.verse-list')!,nodes=Array.from(root.children).map(el=>el.firstChild!);
  const selection=window.getSelection()!,guard=createReadingSelectionGuard();
  const select=(start:Node,from:number,end:Node,to:number)=>selection.setBaseAndExtent(start,from,end,to);
  return {root,nodes,selection,guard,select};
}
it('restores verses 3–4 when the start handle jumps into the chapter heading',()=>{
  const {root,nodes,selection,guard,select}=fixture();
  select(nodes[2],0,nodes[3],10);expect(guard.accept(root,selection)).toBe(true);
  select(document.querySelector('h1')!.firstChild!,0,nodes[3],10);
  expect(selection.toString()).toContain('Verse one');
  expect(guard.accept(root,selection)).toBe(true);
  expect(selection.getRangeAt(0).startContainer).toBe(nodes[2]);
  expect(selection.getRangeAt(0).endContainer).toBe(nodes[3]);
  expect(selection.toString()).toBe('Verse threeVerse four');
});
it('restores backward selection when the end escapes into app navigation',()=>{
  const {root,nodes,selection,guard,select}=fixture();
  select(nodes[3],10,nodes[2],0);guard.accept(root,selection);
  select(document.querySelector('footer')!.firstChild!,10,nodes[2],0);
  guard.accept(root,selection);
  expect(selection.anchorNode).toBe(nodes[3]);expect(selection.focusNode).toBe(nodes[2]);
  expect(selection.toString()).toBe('Verse threeVerse four');
});
it('allows intentional expansion upward and downward within Scripture',()=>{
  const {root,nodes,selection,guard,select}=fixture();
  select(nodes[2],0,nodes[3],10);guard.accept(root,selection);
  select(nodes[0],0,nodes[4],10);expect(guard.accept(root,selection)).toBe(true);
  expect(selection.toString()).toBe('Verse oneVerse twoVerse threeVerse fourVerse five');
});
it('never invents a range for an outside selection or restores disconnected text',()=>{
  const {root,nodes,selection,guard,select}=fixture();
  select(document.querySelector('h1')!.firstChild!,0,nodes[3],10);
  expect(guard.accept(root,selection)).toBe(false);
  select(nodes[2],0,nodes[3],10);guard.accept(root,selection);root.remove();
  select(document.querySelector('h1')!.firstChild!,0,document.querySelector('footer')!.firstChild!,3);
  expect(guard.accept(root,selection)).toBe(false);
});
