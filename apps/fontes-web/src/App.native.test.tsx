// @vitest-environment jsdom
import { afterAll, afterEach, beforeAll, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
vi.mock("./data/preview", () => ({
  books:[{id:1,name:"Genesis",art:"/cover.svg",title:"Beginnings",paragraphs:["A book introduction"]}],
  passages:[{book:1,chapter:1,verses:["In the beginning"],section:"Beginning"}],
  samples:[],loadNotes:()=>[],persistNotes:()=>{},
}));
import App from "./App";
import type { Live } from "./native";
beforeAll(() => vi.stubGlobal("CSS", {escape: (value: string) => value.replace(/[^a-zA-Z0-9_-]/g, "\\$&")}));
beforeAll(() => vi.stubGlobal("matchMedia", (media: string) => ({
  media, matches:false, onchange:null,
  addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent:() => true,
})));
beforeAll(() => vi.stubGlobal("ResizeObserver", class {
  observe() {}
  unobserve() {}
  disconnect() {}
}));
const originalGetAnimations = Element.prototype.getAnimations;
beforeAll(() => {Element.prototype.getAnimations = () => [];});
afterAll(() => {
  if (originalGetAnimations) Element.prototype.getAnimations = originalGetAnimations;
  else delete (Element.prototype as Partial<Element>).getAnimations;
  vi.unstubAllGlobals();
});
afterEach(cleanup);
it("renders the actual native reader before any article or citation is selected", () => {
  const live = {state:{
    content_id:"test",has_study:true,profile:"study",
    session:{selected_ref:1001001,translation:"kjv",preferences:{theme:"terracotta_linen",text_size:24,line_spacing:1.6,verse_numbers:true,scripture_serif:true}},
    books:[{id:1,name:"Genesis",osis:"Gen",verse_counts:[31]}],
    translations:[{id:"kjv",abbr:"KJV",name:"King James Version"}],
    passage:[{id:"v1",refs:[1001001],text:"In the beginning",strongs_links:[],word_links:[]}],
    annotations:[],notes:[],highlights:[],saved:[],
  },send:vi.fn(),error:"",busy:false} as unknown as Live;
  HTMLElement.prototype.scrollTo = vi.fn();
  render(<App live={live}/>);
  expect(screen.getByText("In the beginning")).toBeTruthy();
  expect(screen.getByRole("navigation", {name:"Main navigation"})).toBeTruthy();
});

function wordStudyLive(wordRequest?: (code: string) => Promise<unknown>) {
  const state = {
    content_id:"test",has_study:true,profile:"study",
    session:{selected_ref:1001001,translation:"kjv",preferences:{theme:"terracotta_linen",text_size:24,line_spacing:1.6,verse_numbers:true,scripture_serif:true}},
    books:[{id:1,name:"Genesis",osis:"Gen",verse_counts:[31]}],
    translations:[{id:"kjv",abbr:"KJV",name:"King James Version"}],
    passage:[
      {id:"v1",refs:[1001001],text:"In the beginning",strongs_links:[],word_links:[]},
      {id:"v2",refs:[1001002],text:"The LORD is good",strongs_links:[{start:4,end:8,strongs:"H3068"},{start:12,end:16,strongs:"H2896"}],word_links:[]},
    ],
    annotations:[],notes:[],highlights:[],saved:[],
  };
  const send = vi.fn(async ({command,params}: {command:string;params:Record<string,unknown>}) => {
    if (command === "study.word") return {state,data:wordRequest ? await wordRequest(String(params.code)) : {code:params.code,headwords:[{text:"יְהֹוָה",resource_id:"dictionary"}],definitions:[{id:"dictionary",title:"Definition",excerpt:"The LORD"}]}};
    if (command === "study.resource") return {state,data:{resource:{id:"dictionary",content:"The self-Existent or Eternal",title:"Definition",author:null,source:"Strong’s"}}};
    if (command === "study.occurrences") return {state,data:{occurrences:{total:0,books:[],verses:[],has_more:false,offset:0}}};
    return {state,data:{tokens:[],resources:[]}};
  });
  return {live:{state,send,error:"",busy:false} as unknown as Live,send};
}

it("restores the latest reader position after visiting other panes", () => {
  const {live,send} = wordStudyLive();
  const {container} = render(<App live={live}/>);
  const scroll = container.querySelector<HTMLElement>(".app-scroll")!;
  const navigation = within(screen.getByRole("navigation", {name:"Main navigation"}));
  for (const [index, pane] of ["Study", "Notes", "Library"].entries()) {
    const top = 280 + index * 150;
    scroll.scrollTop = top;
    fireEvent.click(navigation.getByRole("button", {name:pane}));
    expect(scroll.scrollTop).toBe(0);
    scroll.scrollTop = 70;
    fireEvent.click(navigation.getByRole("button", {name:"Read"}));
    expect(scroll.scrollTop).toBe(top);
    // Selecting Read again should also leave the position alone.
    fireEvent.click(navigation.getByRole("button", {name:"Read"}));
    expect(scroll.scrollTop).toBe(top);
  }
  expect(send.mock.calls.some(([request]) => request.command.startsWith("reading."))).toBe(false);
});

it("resets the saved reader position when its chapter, edition or content changes away from Read", () => {
  const {live} = wordStudyLive();
  const {container,rerender} = render(<App live={live}/>);
  const scroll = container.querySelector<HTMLElement>(".app-scroll")!;
  const navigation = within(screen.getByRole("navigation", {name:"Main navigation"}));
  for (const state of [
    {...live.state, session:{...live.state.session, selected_ref:1002001}},
    {...live.state, session:{...live.state.session, translation:"web"}},
    {...live.state, content_id:"replacement"},
  ]) {
    scroll.scrollTop = 420;
    fireEvent.click(navigation.getByRole("button", {name:"Study"}));
    rerender(<App live={{...live,state}}/>);
    fireEvent.click(navigation.getByRole("button", {name:"Read"}));
    expect(scroll.scrollTop).toBe(0);
  }
});

it("opens resources for the tapped verse without changing the reader or its scroll position", async () => {
  const {live} = wordStudyLive();
  const send = vi.fn().mockResolvedValue({state:live.state,data:{resources:[{id:"article",title:"Verse commentary",author:"Commentator",kind:"commentary",basis:"applies_to",excerpt:"About this verse"}],cross_references:[]}});
  const {container} = render(<App live={{...live,send}}/>);
  const reader = container.querySelector(".reader");
  const scroll = container.querySelector<HTMLElement>(".app-scroll")!;
  scroll.scrollTop = 320;
  fireEvent.click(screen.getByRole("button",{name:"Resources for Genesis 1:2"}));
  const panel = await screen.findByRole("dialog",{name:"Genesis 1:2"});
  fireEvent.click(await within(panel).findByRole("button",{name:/Commentary/}));
  await within(panel).findByText("Verse commentary");
  expect(send).toHaveBeenCalledExactlyOnceWith({command:"study.context",params:{ref_id:1001002}});
  fireEvent.click(within(panel).getByRole("button",{name:"Close"}));
  await waitFor(()=>expect(screen.queryByRole("dialog")).toBeNull());
  expect(container.querySelector(".reader")).toBe(reader);
  expect(scroll.scrollTop).toBe(320);
});

it("layers study symbols over a normal highlight and saves/removes only the chosen layer", async () => {
  const {live} = wordStudyLive();
  const text = live.state.passage[0].text;
  const selection = {translation_id:"kjv",spans:[{verse_id:live.state.passage[0].id,start:0,end:Array.from(text).length,quote:text}]};
  live.state.annotations = ["normal","study"].map(id=>({id,kind:"highlight",start_ref:1001001,end_ref:1001001,content:null,color:"yellow",updated_at:""}));
  live.state.highlights = [
    {annotation_id:"study",style:{kind:"symbol",color:"blue",symbol:"God"},selection},
    {annotation_id:"normal",style:{kind:"highlighter",color:"yellow",symbol:null},selection},
  ];
  const send = vi.fn().mockResolvedValue({state:live.state,data:{}});
  const {container,rerender} = render(<App live={{...live,send}}/>);
  const verse = container.querySelector<HTMLElement>('[data-reference="Genesis 1:1"]')!;
  expect(verse.querySelector('[data-highlight-kind="Highlighter"] [data-highlight-kind="Study symbol"]')).toBeTruthy();
  expect(verse.querySelector('[data-verse-text]')?.textContent).toBe(text);
  window.getSelection()?.removeAllRanges();
  async function picker() {
    fireEvent.contextMenu(verse);
    fireEvent.click(await screen.findByRole("button",{name:/Highlight/}));
    const panel = await screen.findByRole("dialog",{name:"Highlight style"});
    fireEvent.click(within(panel).getByRole("tab",{name:"Study symbols"}));
    fireEvent.click(within(panel).getByRole("button",{name:/God/}));
    return panel;
  }
  let panel = await picker();
  fireEvent.click(within(panel).getByRole("button",{name:"Remove highlight"}));
  const layers = await screen.findByRole("dialog", {name:"Remove a highlight"});
  expect(send).not.toHaveBeenCalled();
  fireEvent.click(within(layers).getByRole("button", {name:"Remove Study symbol: God · Blue"}));
  await waitFor(()=>expect(send).toHaveBeenCalledWith({command:"annotation.delete",params:{id:"study"}}));
  live.state.highlights = live.state.highlights.filter(h=>h.annotation_id !== "study");
  rerender(<App live={{...live,send}}/>);
  panel = await picker();
  fireEvent.click(within(panel).getByRole("button",{name:"Apply"}));
  await waitFor(()=>expect(send).toHaveBeenCalledWith({command:"highlight.save",params:expect.objectContaining({id:null,selection,style:expect.objectContaining({kind:"symbol",symbol:"God"})})}));
  expect(live.state.highlights[0].annotation_id).toBe("normal");
});

it("removes a partially reselected non-default highlight and keeps failures retryable", async () => {
  const {live} = wordStudyLive();
  const text = live.state.passage[0].text;
  live.state.annotations = [{id:"underline",kind:"highlight",start_ref:1001001,end_ref:1001001,content:null,color:"blue",updated_at:""}];
  live.state.highlights = [{annotation_id:"underline",style:{kind:"underline",color:"blue",symbol:null},selection:{translation_id:"kjv",spans:[{verse_id:live.state.passage[0].id,start:0,end:Array.from(text).length,quote:text}]}}];
  const send = vi.fn().mockRejectedValueOnce(Error("Could not remove highlight")).mockResolvedValue({state:live.state,data:{}});
  const {container} = render(<App live={{...live,send}}/>);
  const verse = container.querySelector<HTMLElement>('[data-reference="Genesis 1:1"]')!;
  const node = document.createTreeWalker(verse.querySelector('[data-verse-text]')!,NodeFilter.SHOW_TEXT).nextNode()!;
  const range = document.createRange();range.setStart(node,3);range.setEnd(node,6);
  window.getSelection()?.removeAllRanges();window.getSelection()?.addRange(range);
  fireEvent.contextMenu(verse);
  fireEvent.click(await screen.findByRole("button",{name:/Highlight/}));
  const panel = await screen.findByRole("dialog",{name:"Highlight style"});
  fireEvent.click(within(panel).getByRole("button",{name:"Remove highlight"}));
  await within(panel).findByRole("alert");
  expect(send).toHaveBeenCalledWith({command:"annotation.delete",params:{id:"underline"}});
  expect(screen.queryByRole("dialog",{name:"Remove a highlight"})).toBeNull();
  fireEvent.click(within(panel).getByRole("button",{name:"Remove highlight"}));
  await waitFor(()=>expect(screen.queryByRole("dialog")).toBeNull());
  expect(send).toHaveBeenCalledTimes(2);
  expect(send.mock.calls.every(([r])=>r.command === "annotation.delete")).toBe(true);
});

it("explains when no saved highlight overlaps instead of silently closing the picker", async () => {
  const {live,send} = wordStudyLive();
  const {container} = render(<App live={live}/>);
  window.getSelection()?.removeAllRanges();
  fireEvent.contextMenu(container.querySelector('[data-reference="Genesis 1:1"]')!);
  fireEvent.click(await screen.findByRole("button",{name:/Highlight/}));
  const panel = await screen.findByRole("dialog",{name:"Highlight style"});
  fireEvent.click(within(panel).getByRole("button",{name:"Remove highlight"}));
  expect(within(panel).getByRole("alert").textContent).toContain("No saved highlights overlap");
  expect(send).not.toHaveBeenCalled();
});

function highlightedLive() {
  const {live} = wordStudyLive();
  live.state.annotations = [{id:"highlight", kind:"highlight",start_ref:1001002,end_ref:1001003,content:null,updated_at:"",color:"yellow"}];
  return live;
}

async function openNotebookHighlights() {
  const navigation = within(screen.getByRole("navigation", {name:"Main navigation"}));
  fireEvent.click(navigation.getByRole("button", {name:"Notes"}));
  fireEvent.click(screen.getByRole("tab", {name:"Highlights"}));
  return screen.findByRole("button", {name:"Read Genesis 1:2"});
}

it("opens a notebook highlight at its first verse after navigation succeeds", async () => {
  const live = highlightedLive();
  let resolve!: (value: unknown) => void;
  const pending = new Promise(r => {resolve = r;});
  const send = vi.fn(() => pending);
  const scrollIntoView = vi.fn();
  HTMLElement.prototype.scrollIntoView = scrollIntoView;
  const {container} = render(<App live={{...live,send} as Live}/>);
  container.querySelector<HTMLElement>(".app-scroll")!.scrollTop = 420;
  fireEvent.click(await openNotebookHighlights());
  expect(send).toHaveBeenCalledWith({command:"reading.select",params:{ref_id:1001002}});
  expect(container.querySelector(".reader")).toBeNull();
  await act(async () => resolve({state:live.state,data:{}}));
  await waitFor(() => expect(scrollIntoView).toHaveBeenCalledWith({block:"center",behavior:"instant"}));
  expect(container.querySelector('[data-verse-refs="1001002"]')?.classList.contains("verse-arrival")).toBe(true);
  expect(send).toHaveBeenCalledTimes(1);
});

it("keeps the highlight list open when navigation fails and removes without navigating", async () => {
  const live = highlightedLive();
  const send = vi.fn().mockRejectedValueOnce(Error("Unable to open passage")).mockResolvedValue({state:live.state,data:{}});
  const {container} = render(<App live={{...live,send} as Live}/>);
  fireEvent.click(await openNotebookHighlights());
  await screen.findByText("Error: Unable to open passage");
  expect(container.querySelector(".reader")).toBeNull();
  fireEvent.click(screen.getByRole("button", {name:"Remove highlight"}));
  await waitFor(() => expect(send).toHaveBeenLastCalledWith({command:"annotation.delete",params:{id:"highlight"}}));
  expect(send).toHaveBeenCalledTimes(2);
});

it("opens word study over the same reader and closes without resetting scroll or navigating", async () => {
  const {live,send} = wordStudyLive();
  const scrollTo = vi.fn();
  HTMLElement.prototype.scrollTo = scrollTo;
  const {container} = render(<App live={live}/>);
  const reader = container.querySelector(".reader");
  const scroll = container.querySelector(".app-scroll")!;
  scroll.scrollTop = 280;
  scrollTo.mockClear();
  fireEvent.click(screen.getByRole("button",{name:"Study LORD"}));
  const panel = await screen.findByRole("dialog",{name:/Word study/});
  await within(panel).findByRole("button",{name:"Show definition"});
  expect(container.querySelector(".reader")).toBe(reader);
  expect(scroll.scrollTop).toBe(280);
  expect(send.mock.calls.some(([request]) => request.command.startsWith("reading."))).toBe(false);
  expect(send.mock.calls.some(([r]) => r.command === "study.context")).toBe(false);
  fireEvent.click(within(panel).getByText("Words in Genesis 1:2"));
  await waitFor(() => expect(send).toHaveBeenCalledWith({command:"study.context",params:{ref_id:1001002}}));
  // Focus or nested scrolling in a WebView must not change the return position.
  scroll.scrollTop = 120;
  fireEvent.click(within(panel).getByRole("button",{name:"Close word study"}));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(container.querySelector(".reader")).toBe(reader);
  expect(scroll.scrollTop).toBe(280);
  expect(scrollTo).not.toHaveBeenCalled();
});

it("can dismiss a pending lookup and ignores its reply after another word opens", async () => {
  let resolveFirst!: (data: unknown) => void;
  const first = new Promise(resolve => {resolveFirst = resolve;});
  const {live,send} = wordStudyLive(async code => code === "H3068" ? first : {code,headwords:[{text:"Good headword",resource_id:"dictionary"}],definitions:[]});
  render(<App live={live}/>);
  fireEvent.click(screen.getByRole("button",{name:"Study LORD"}));
  const panel = await screen.findByRole("dialog",{name:/Word study/});
  expect(within(panel).getByRole("status").textContent).toContain("Loading");
  fireEvent.click(within(panel).getByRole("button",{name:"Close word study"}));
  fireEvent.click(await screen.findByRole("button",{name:"Study good"}));
  await screen.findByText("Good headword");
  await act(async () => resolveFirst({code:"H3068",headwords:[{text:"Stale headword"}],definitions:[{id:"stale"}]}));
  expect(screen.queryByText("Stale headword")).toBeNull();
  expect(send.mock.calls.some(([r]) => r.command === "study.resource")).toBe(false);
});

it("keeps lookup errors dismissible and closes the panel when collecting a note", async () => {
  const request = vi.fn().mockRejectedValueOnce(Error("Dictionary unavailable")).mockResolvedValue({code:"H3068",headwords:[],definitions:[{id:"dictionary",title:"Definition",excerpt:"The LORD"}]});
  const {live} = wordStudyLive(request);
  render(<App live={live}/>);
  fireEvent.click(screen.getByRole("button",{name:"Study LORD"}));
  const panel = await screen.findByRole("dialog",{name:/Word study/});
  await within(panel).findByRole("alert");
  fireEvent.click(within(panel).getByRole("button",{name:"Close word study"}));
  fireEvent.click(await screen.findByRole("button",{name:"Study LORD"}));
  fireEvent.click(await screen.findByRole("button",{name:"Collect in a note"}));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(screen.getByPlaceholderText("Give this thought a title")).toBeTruthy();
});


it("shows the headword while its definition is pending and queues no secondary lookups", async () => {
  const {live,send} = wordStudyLive();
  const implementation = send.getMockImplementation()!;
  const response = await implementation({command:"study.resource",params:{id:"dictionary"}});
  let resolveResource!: (value: typeof response) => void;
  const resource = new Promise<typeof response>(resolve => {resolveResource=resolve;});
  send.mockImplementation(request => request.command === "study.resource" ? resource : implementation(request));
  render(<App live={live}/>);
  fireEvent.click(screen.getByRole("button",{name:"Study LORD"}));
  const panel=await screen.findByRole("dialog",{name:/Word study/});
  await within(panel).findByText("יְהֹוָה");
  expect(send.mock.calls.map(([r])=>r.command)).toEqual(["study.word","study.resource"]);
  expect((within(panel).getByRole("button",{name:"Collect in a note"}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(within(panel).getByRole("button",{name:"Show definition"}));
  expect(within(panel).getByRole("status").textContent).toContain("Loading definition");
  await act(async () => resolveResource(response));
  await within(panel).findByText("The self-Existent or Eternal");
  expect((within(panel).getByRole("button",{name:"Collect in a note"}) as HTMLButtonElement).disabled).toBe(false);
});

it("loads occurrences only on demand and keeps a failure separate from Meaning", async () => {
  const {live,send}=wordStudyLive();
  const implementation=send.getMockImplementation()!;
  let occurrenceCalls=0;
  send.mockImplementation(async request => {
    if (request.command === "study.occurrences" && occurrenceCalls++ === 0) throw Error("Occurrence lookup failed");
    return implementation(request);
  });
  render(<App live={live}/>);
  fireEvent.click(screen.getByRole("button",{name:"Study LORD"}));
  const panel=await screen.findByRole("dialog",{name:/Word study/});
  await within(panel).findByRole("button",{name:"Show definition"});
  expect(occurrenceCalls).toBe(0);
  fireEvent.click(within(panel).getByRole("tab",{name:"Occurrences"}));
  await within(panel).findByText(/Occurrence lookup failed/);
  fireEvent.click(within(panel).getByRole("button",{name:"Retry occurrences"}));
  await within(panel).findByText("A word across the books");
  expect(occurrenceCalls).toBe(2);
  fireEvent.click(within(panel).getByRole("tab",{name:"Meaning"}));
  expect(within(panel).queryByRole("alert")).toBeNull();
  expect(within(panel).getByRole("button",{name:"Show definition"})).toBeTruthy();
});

it("reuses dictionary reads within a snapshot and reloads after the snapshot changes", async () => {
  const {live,send}=wordStudyLive();
  const {rerender}=render(<App live={live}/>);
  async function openAndClose() {
    fireEvent.click(await screen.findByRole("button",{name:"Study LORD"}));
    const panel=await screen.findByRole("dialog",{name:/Word study/});
    await waitFor(() => expect((within(panel).getByRole("button",{name:"Collect in a note"}) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(within(panel).getByRole("button",{name:"Close word study"}));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  }
  await openAndClose();
  await openAndClose();
  expect(send.mock.calls.filter(([r])=>r.command === "study.word")).toHaveLength(1);
  expect(send.mock.calls.filter(([r])=>r.command === "study.resource")).toHaveLength(1);
  live.state.content_id="new-snapshot";
  rerender(<App live={{...live}}/>);
  await openAndClose();
  expect(send.mock.calls.filter(([r])=>r.command === "study.word")).toHaveLength(2);
  expect(send.mock.calls.filter(([r])=>r.command === "study.resource")).toHaveLength(2);
});


it("shows actionable study guidance instead of a headword card before selecting a word", () => {
  const {live} = wordStudyLive();
  const {container} = render(<App live={live}/>);
  fireEvent.click(within(screen.getByRole("navigation", {name:"Main navigation"})).getByRole("button", {name:"Study"}));
  const guidance = screen.getByRole("region", {name:"Choose a word to study"});
  expect(within(guidance).getByRole("heading", {name:"Choose a word to study"})).toBeTruthy();
  expect(container.querySelector(".word-card")).toBeNull();
  expect(screen.queryByRole("tab", {name:"Occurrences"})).toBeNull();
  fireEvent.click(within(guidance).getByRole("button", {name:"Go to Read"}));
  expect(screen.getByRole("button", {name:"Study LORD"})).toBeTruthy();
});

it("opens an occurrence in Read, closing word study only after successful navigation", async () => {
  const {live,send} = wordStudyLive();
  const implementation = send.getMockImplementation()!;
  let navigationCalls = 0;
  send.mockImplementation(async request => {
    if (request.command === "study.occurrences") return {state:live.state,data:{occurrences:{total:1,books:[{book_id:2,label:"Exodus",verses:1}],verses:request.params.book_id ? [{ref_id:2004001,label:"Exodus 4:1"}] : [],has_more:false,offset:0}}} as never;
    if (request.command === "reading.preview") return {state:live.state,data:{verses:[{text:"And Moses answered and said"}]}} as never;
    if (request.command === "reading.select" && navigationCalls++ === 0) throw Error("Could not open verse");
    return implementation(request);
  });
  render(<App live={live}/>);
  fireEvent.click(screen.getByRole("button",{name:"Study LORD"}));
  const panel = await screen.findByRole("dialog",{name:/Word study/});
  await within(panel).findByRole("button",{name:"Show definition"});
  fireEvent.click(within(panel).getByRole("tab",{name:"Occurrences"}));
  fireEvent.click(await within(panel).findByRole("button",{name:"Exodus 1 verses"}));
  fireEvent.click(await within(panel).findByRole("button",{name:"Exodus 4:1"}));
  await within(panel).findByText("And Moses answered and said");
  expect(send.mock.calls.some(([r])=>r.command === "reading.select")).toBe(false);
  fireEvent.click(within(panel).getByRole("button",{name:"Open Exodus 4:1 in Read"}));
  await within(panel).findByText(/Could not open verse/);
  expect(screen.getByRole("dialog",{name:/Word study/})).toBeTruthy();
  fireEvent.click(within(panel).getByRole("button",{name:"Retry opening verse"}));
  await waitFor(()=>expect(screen.queryByRole("dialog",{name:/Word study/})).toBeNull());
  expect(send.mock.calls.filter(([r])=>r.command === "reading.select").map(([r])=>r.params)).toEqual([{ref_id:2004001},{ref_id:2004001}]);
  expect(send.mock.calls.filter(([r])=>r.command === "reading.preview").map(([r])=>r.params)).toEqual([{start_ref:2004001,end_ref:2004001}]);
  expect(screen.getByRole("button",{name:"Study LORD"})).toBeTruthy();
});

it("offers a share card for the selected Scripture with its reference and edition",async()=>{
  const {live,send}=wordStudyLive();
  const {container}=render(<App live={live}/>);
  window.getSelection()?.removeAllRanges();
  fireEvent.contextMenu(container.querySelector<HTMLElement>('[data-reference="Genesis 1:1"]')!);
  fireEvent.click(await screen.findByRole("button",{name:"Share passage"}));
  const panel=await screen.findByRole("dialog",{name:"Share a passage"});
  expect(panel.querySelector("canvas")?.getAttribute("aria-label")).toBe("Share card: In the beginning — Genesis 1:1. King James Version (KJV)");
  expect(within(panel).getByRole("button",{name:"Quiet harbor"})).toBeTruthy();
  expect(within(panel).getByRole("button",{name:"Olive hills"})).toBeTruthy();
  expect(send).not.toHaveBeenCalled();
});


it("does not invent source attribution when the active translation is missing",async()=>{
  const {live}=wordStudyLive();live.state.translations=[];
  const {container}=render(<App live={live}/>);
  window.getSelection()?.removeAllRanges();
  fireEvent.contextMenu(container.querySelector<HTMLElement>('[data-reference="Genesis 1:1"]')!);
  fireEvent.click(await screen.findByRole("button",{name:"Share passage"}));
  await screen.findByText("The selected translation's attribution is unavailable.");
  expect(screen.queryByRole("dialog",{name:"Share a passage"})).toBeNull();
});


it("clears native selection handles when sharing while retaining the selected quotation",async()=>{
  const {live}=wordStudyLive();
  const {container}=render(<App live={live}/>);
  const verse=container.querySelector<HTMLElement>('[data-reference="Genesis 1:1"]')!;
  const node=document.createTreeWalker(verse.querySelector('[data-verse-text]')!,NodeFilter.SHOW_TEXT).nextNode()!;
  const range=document.createRange();range.setStart(node,3);range.setEnd(node,6);
  const selection=window.getSelection()!;selection.removeAllRanges();selection.addRange(range);
  fireEvent.contextMenu(verse);
  expect(selection.toString()).toBe("the");
  fireEvent.click(await screen.findByRole("button",{name:"Share passage"}));
  const panel=await screen.findByRole("dialog",{name:"Share a passage"});
  expect(selection.rangeCount).toBe(0);
  expect(panel.querySelector("canvas")?.getAttribute("aria-label")).toBe("Share card: the — Genesis 1:1. King James Version (KJV)");
  // Selection inside an already-open sheet remains available for ordinary copy.
  const inside=document.createRange();inside.selectNodeContents(within(panel).getByRole("heading",{name:"Share a passage"}));
  selection.addRange(inside);
  fireEvent.click(within(panel).getByRole("button",{name:"Olive hills"}));
  expect(selection.toString()).toBe("Share a passage");
  selection.removeAllRanges();
  fireEvent.click(within(panel).getByRole("button",{name:"Close"}));
  await waitFor(() => expect(screen.queryByRole("dialog",{name:"Share a passage"})).toBeNull());
});

it("keeps exact repeated-word highlight spans after the sheet clears the DOM selection",async()=>{
  const {live,send}=wordStudyLive();live.state.passage[0].text="word word";
  const {container}=render(<App live={live}/>);
  const verse=container.querySelector<HTMLElement>('[data-reference="Genesis 1:1"]')!;
  const node=document.createTreeWalker(verse.querySelector('[data-verse-text]')!,NodeFilter.SHOW_TEXT).nextNode()!;
  const range=document.createRange();range.setStart(node,5);range.setEnd(node,9);
  const selection=window.getSelection()!;selection.removeAllRanges();selection.addRange(range);
  fireEvent.contextMenu(verse);
  fireEvent.click(await screen.findByRole("button",{name:/Highlight/}));
  const panel=await screen.findByRole("dialog",{name:"Highlight style"});
  await waitFor(()=>expect(selection.rangeCount).toBe(0));
  fireEvent.click(within(panel).getByRole("button",{name:"Apply"}));
  await waitFor(()=>expect(send.mock.calls.some(([r])=>r.command==="highlight.save")).toBe(true));
  const saved=send.mock.calls.find(([r])=>r.command==="highlight.save")![0];
  expect(saved.params.selection).toEqual({translation_id:"kjv",spans:[{verse_id:"v1",start:5,end:9,quote:"word"}]});
});

it("keeps reader nodes and range endpoints stable while handles move, then captures the latest range",async()=>{
  const {live}=wordStudyLive();
  live.state.passage.push({id:"v3",refs:[1001003],text:"Third verse text",strongs_links:[],word_links:[]});
  const {container}=render(<App live={live}/>);
  const reader=container.querySelector('.reader')!;
  const verses=Array.from(reader.querySelectorAll('[data-verse-text]'));
  const firstText=(el:Element)=>document.createTreeWalker(el,NodeFilter.SHOW_TEXT).nextNode()!;
  const start=firstText(verses[0]),end=firstText(verses[2]);
  const range=document.createRange();range.setStart(start,3);range.setEnd(end,5);
  const domSelection=window.getSelection()!;domSelection.removeAllRanges();domSelection.addRange(range);
  const mutations:MutationRecord[]=[];
  const observer=new MutationObserver(records=>mutations.push(...records));observer.observe(reader,{subtree:true,childList:true,characterData:true});
  fireEvent(document,new Event('selectionchange'));
  await screen.findByRole('button',{name:'Share passage'});
  expect(container.querySelector('.reader')).toBe(reader);
  expect(domSelection.getRangeAt(0).startContainer).toBe(start);
  expect(domSelection.getRangeAt(0).endContainer).toBe(end);
  expect(mutations).toHaveLength(0);
  // Change the handles and immediately press Share before the UI debounce finishes.
  range.setStart(end,0);range.setEnd(end,5);
  fireEvent(document,new Event('selectionchange'));
  fireEvent.click(screen.getByRole('button',{name:'Share passage'}));
  const panel=await screen.findByRole('dialog',{name:'Share a passage'});
  expect(panel.querySelector('canvas')?.getAttribute('aria-label')).toBe('Share card: Third — Genesis 1:3. King James Version (KJV)');
  observer.disconnect();
});


it("restores a 3–4 selection that Android expands into chapter chrome before sharing",async()=>{
  const {live}=wordStudyLive();
  live.state.passage.push({id:"v3",refs:[1001003],text:"Third verse",strongs_links:[],word_links:[]},{id:"v4",refs:[1001004],text:"Fourth verse",strongs_links:[],word_links:[]});
  const {container}=render(<App live={live}/>);
  const text=(el:Element)=>document.createTreeWalker(el,NodeFilter.SHOW_TEXT).nextNode()!;
  const verses=Array.from(container.querySelectorAll('[data-verse-text]'));
  const start=text(verses[2]),end=text(verses[3]),selection=window.getSelection()!;
  selection.setBaseAndExtent(start,0,end,12);
  fireEvent(document,new Event('selectionchange'));
  await screen.findByRole('button',{name:'Share passage'});
  selection.setBaseAndExtent(text(container.querySelector('.chapter-heading')!),0,end,12);
  expect(selection.toString()).toContain('In the beginning');
  fireEvent(document,new Event('selectionchange'));
  expect(selection.anchorNode).toBe(start);expect(selection.focusNode).toBe(end);
  fireEvent.click(screen.getByRole('button',{name:'Share passage'}));
  const panel=await screen.findByRole('dialog',{name:'Share a passage'});
  expect(panel.querySelector('canvas')?.getAttribute('aria-label')).toBe('Share card: Third verse Fourth verse — Genesis 1:3 – Genesis 1:4. King James Version (KJV)');
});
