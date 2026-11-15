// Preserve the last intentional Scripture range if Android's handle/autoscroll
// selection escapes into chapter chrome or another part of the app.
export function createReadingSelectionGuard() {
  let saved: {range:Range;backward:boolean;root:HTMLElement}|null=null;
  function inside(root:HTMLElement,range:Range) {
    if(!root.isConnected || !root.contains(range.startContainer) || !root.contains(range.endContainer))return false;
    const bounds=document.createRange();bounds.selectNodeContents(root);
    return range.compareBoundaryPoints(Range.START_TO_START,bounds)>=0 && range.compareBoundaryPoints(Range.END_TO_END,bounds)<=0;
  }
  return {
    reset(){saved=null;},
    accept(root:HTMLElement,selection:Selection):boolean {
      if(!selection.rangeCount || selection.isCollapsed){saved=null;return false;}
      const range=selection.getRangeAt(0);
      if(inside(root,range)) {
        saved={root,range:range.cloneRange(),backward:selection.anchorNode===range.endContainer && selection.anchorOffset===range.endOffset};
        return true;
      }
      if(!saved || saved.root!==root || !inside(root,saved.range)){saved=null;return false;}
      const last=saved.range;
      selection.setBaseAndExtent(saved.backward?last.endContainer:last.startContainer,saved.backward?last.endOffset:last.startOffset,saved.backward?last.startContainer:last.endContainer,saved.backward?last.startOffset:last.endOffset);
      return true;
    },
  };
}
