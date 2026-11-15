// @vitest-environment jsdom
import {afterAll,beforeAll,afterEach,expect,it,vi} from "vitest";
import {act,cleanup,fireEvent,render,screen,waitFor} from "@testing-library/react";
const api=vi.hoisted(()=>({renderShareCard:vi.fn(),cardFile:vi.fn(),shareCard:vi.fn(),downloadCard:vi.fn()}));
vi.mock("../shareCard",()=>({...api,cardBackgrounds:[{id:"harbor",name:"Quiet harbor",url:"/harbor.jpg"},{id:"hills",name:"Olive hills",url:"/hills.jpg"}]}));
vi.mock("@tauri-apps/api/core",()=>({isTauri:()=>true}));
import {ShareCard} from "./ShareCard";
beforeAll(()=>{
  vi.stubGlobal("CSS",{escape:(value:string)=>value.replace(/[^a-zA-Z0-9_-]/g,"_")});
  vi.stubGlobal("matchMedia",()=>({matches:false,addListener(){},removeListener(){},addEventListener(){},removeEventListener(){}}));
  vi.stubGlobal("ResizeObserver",class{observe(){}unobserve(){}disconnect(){}});
});
const originalAnimations=Element.prototype.getAnimations;
beforeAll(()=>{Element.prototype.getAnimations=()=>[];});
afterAll(()=>{vi.unstubAllGlobals();if(originalAnimations)Element.prototype.getAnimations=originalAnimations;else delete (Element.prototype as Partial<Element>).getAnimations;});
const quote={text:"A complete quotation",reference:"Genesis 1:1",source:"King James Version (KJV)"};
afterEach(()=>{cleanup();vi.restoreAllMocks();vi.resetAllMocks();});
function setup() {
  vi.spyOn(HTMLCanvasElement.prototype,"getContext").mockReturnValue({drawImage:vi.fn()} as never);
  const canvas=document.createElement("canvas");
  canvas.width=1080;canvas.height=1350;
  canvas.toDataURL=()=>"data:image/png;base64,eA==";
  const file=new File(["png"],"card.png",{type:"image/png"});
  api.cardFile.mockResolvedValue(file);api.renderShareCard.mockResolvedValue(canvas);
  return {canvas,file};
}
it("disables sharing while an updated background renders and shares the finished card",async()=>{
  const {canvas,file}=setup();
  render(<ShareCard quote={quote} onClose={()=>{}}/>);
  await waitFor(()=>expect((screen.getByRole("button",{name:"Share image"}) as HTMLButtonElement).disabled).toBe(false));
  let finish!:(value:HTMLCanvasElement)=>void;
  api.renderShareCard.mockReturnValueOnce(new Promise(resolve=>{finish=resolve;}));
  fireEvent.click(screen.getByRole("button",{name:"Olive hills"}));
  expect((screen.getByRole("button",{name:"Share image"}) as HTMLButtonElement).disabled).toBe(true);
  await act(async()=>finish(canvas));
  fireEvent.click(screen.getByRole("button",{name:"Share image"}));
  await waitFor(()=>expect(api.shareCard).toHaveBeenCalledExactlyOnceWith(file,"data:image/png;base64,eA==",quote));
});
it("keeps failed or oversized renders unshareable and allows retry",async()=>{
  setup();api.renderShareCard.mockRejectedValueOnce(Error("Selection is too long"));
  render(<ShareCard quote={quote} onClose={()=>{}}/>);
  await screen.findByText(/Selection is too long/);
  expect((screen.getByRole("button",{name:"Share image"}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button",{name:"Retry card"}));
  await waitFor(()=>expect((screen.getByRole("button",{name:"Share image"}) as HTMLButtonElement).disabled).toBe(false));
});
