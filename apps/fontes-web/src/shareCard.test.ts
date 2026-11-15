// @vitest-environment jsdom
import {afterEach,expect,it,vi} from "vitest";
const native=vi.hoisted(()=>({isTauri:vi.fn(()=>false),invoke:vi.fn()}));
vi.mock("@tauri-apps/api/core",()=>native);
import {cardSize,shareCard,wrapCardText} from "./shareCard";
afterEach(()=>{vi.restoreAllMocks();native.isTauri.mockReturnValue(false);native.invoke.mockReset();});
it("wraps explicit lines and unbroken Unicode text without losing characters",()=>{
  const measure=(text:string)=>Array.from(text).length;
  expect(wrapCardText("In the beginning\nGod created",measure,12)).toEqual(["In the","beginning","God created"]);
  const text="𐀀abcdefghijk";
  const lines=wrapCardText(text,measure,4);
  expect(lines.join("")).toBe(text);expect(lines.every(l=>measure(l)<=4)).toBe(true);
  expect(cardSize("portrait")).toEqual({width:1080,height:1350});
  expect(cardSize("square")).toEqual({width:1080,height:1080});
});
it("shares the prepared PNG and its full attribution through the native bridge",async()=>{
  native.isTauri.mockReturnValue(true);
  const quote={text:"In the beginning",reference:"Genesis 1:1",source:"King James Version (KJV)"};
  await shareCard(new File(["png"],"card.png",{type:"image/png"}),"data:image/png;base64,aGVsbG8=",quote);
  expect(native.invoke).toHaveBeenCalledExactlyOnceWith("share_verse_image",{dataUrl:"data:image/png;base64,aGVsbG8=",caption:"“In the beginning”\n— Genesis 1:1\nKing James Version (KJV)\nShared with Fontes"});
});
it("uses browser file sharing without stripping the source attribution",async()=>{
  const share=vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator,"canShare",{value:()=>true,configurable:true});
  Object.defineProperty(navigator,"share",{value:share,configurable:true});
  const file=new File(["png"],"card.png",{type:"image/png"});
  await shareCard(file,"data:image/png;base64,eA==",{text:"A quotation",reference:"Genesis 1:1",source:"King James Version (KJV)"});
  expect(share).toHaveBeenCalledWith(expect.objectContaining({files:[file],text:expect.stringContaining("King James Version (KJV)")}));
  expect(native.invoke).not.toHaveBeenCalled();
});
