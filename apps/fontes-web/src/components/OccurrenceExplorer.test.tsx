// @vitest-environment jsdom
import {afterEach,expect,it,vi} from "vitest";
import {cleanup,fireEvent,render,screen} from "@testing-library/react";
import {OccurrenceExplorer} from "./OccurrenceExplorer";
afterEach(cleanup);
it("previews a verse without navigating, with a separate Open in Read action",async()=>{
  const page=vi.fn(async()=>({verses:[{ref_id:1001001,label:"Genesis 1:1"}],has_more:false}));
  const verse=vi.fn(async()=>{});
  render(<OccurrenceExplorer books={[{id:1,name:"Genesis",verses:141}]} loadPage={page} loadVerse={async()=>"In the beginning"} onRead={verse}/>);
  expect(page).not.toHaveBeenCalled();expect(verse).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button",{name:"Genesis 141 verses"}));
  const reference=await screen.findByRole("button",{name:"Genesis 1:1"});
  expect(page).toHaveBeenCalledExactlyOnceWith(1,0);expect(verse).not.toHaveBeenCalled();
  expect(reference.closest(".occurrence-book-section")?.textContent).toContain("Genesis 141 verses");
  fireEvent.click(reference);
  await screen.findByText("In the beginning");
  expect(verse).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button",{name:"Open Genesis 1:1 in Read"}));
  await vi.waitFor(()=>expect(verse).toHaveBeenCalledExactlyOnceWith(1001001));
  fireEvent.click(screen.getByRole("button",{name:"Genesis 141 verses"}));
  expect(screen.queryByRole("button",{name:"Genesis 1:1"})).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Genesis 141 verses"}));
  expect(page).toHaveBeenCalledTimes(1);
});
it("appends more reference headings in place without expanding verse bodies",async()=>{
  const page=vi.fn(async(_book:number,offset:number)=>({verses:[{ref_id:1001001+offset,label:`Genesis 1:${1+offset}`}],has_more:offset===0}));
  const verse=vi.fn(async()=>{});
  render(<OccurrenceExplorer books={[{id:1,name:"Genesis",verses:2}]} loadPage={page} loadVerse={async()=>"In the beginning"} onRead={verse}/>);
  fireEvent.click(screen.getByRole("button",{name:"Genesis 2 verses"}));
  fireEvent.click(await screen.findByRole("button",{name:"Show more references in Genesis"}));
  await screen.findByRole("button",{name:"Genesis 1:2"});
  expect(screen.getByRole("button",{name:"Genesis 1:1"})).toBeTruthy();
  expect(page).toHaveBeenLastCalledWith(1,1);expect(verse).not.toHaveBeenCalled();
});
it("allows retrying a failed reference request in the expanded book",async()=>{
  const page=vi.fn().mockRejectedValueOnce(Error("Offline")).mockResolvedValueOnce({verses:[{ref_id:1001001,label:"Genesis 1:1"}],has_more:false});
  render(<OccurrenceExplorer books={[{id:1,name:"Genesis",verses:1}]} loadPage={page} loadVerse={async()=>"In the beginning"} onRead={async()=>{}}/>);
  fireEvent.click(screen.getByRole("button",{name:"Genesis 1 verses"}));
  fireEvent.click(await screen.findByRole("button",{name:"Retry references"}));
  await screen.findByRole("button",{name:"Genesis 1:1"});
  expect(screen.queryByRole("alert")).toBeNull();
});


it("retains the reference and retries failed navigation",async()=>{
  const read=vi.fn().mockRejectedValueOnce(Error("Verse unavailable")).mockResolvedValueOnce(undefined);
  render(<OccurrenceExplorer books={[{id:1,name:"Genesis",verses:1}]} loadPage={async()=>({verses:[{ref_id:1001001,label:"Genesis 1:1"}],has_more:false})} loadVerse={async()=>"In the beginning"} onRead={read}/>);
  fireEvent.click(screen.getByRole("button",{name:"Genesis 1 verses"}));
  fireEvent.click(await screen.findByRole("button",{name:"Genesis 1:1"}));
  fireEvent.click(screen.getByRole("button",{name:"Open Genesis 1:1 in Read"}));
  await screen.findByText(/Verse unavailable/);
  fireEvent.click(screen.getByRole("button",{name:"Retry opening verse"}));
  await vi.waitFor(()=>expect(read).toHaveBeenCalledTimes(2));
  await vi.waitFor(()=>expect(screen.queryByRole("alert")).toBeNull());
  expect(read).toHaveBeenLastCalledWith(1001001);
});

it("retries preview errors and keeps opening in Read available",async()=>{
  const preview=vi.fn().mockRejectedValueOnce(Error("Preview failed")).mockResolvedValueOnce("Restored verse text");
  const read=vi.fn(async()=>{});
  render(<OccurrenceExplorer books={[{id:1,name:"Genesis",verses:1}]} loadPage={async()=>({verses:[{ref_id:1001001,label:"Genesis 1:1"}],has_more:false})} loadVerse={preview} onRead={read}/>);
  fireEvent.click(screen.getByRole("button",{name:"Genesis 1 verses"}));
  fireEvent.click(await screen.findByRole("button",{name:"Genesis 1:1"}));
  await screen.findByText(/Preview failed/);
  expect(screen.getByRole("button",{name:"Open Genesis 1:1 in Read"})).toBeTruthy();
  fireEvent.click(screen.getByRole("button",{name:"Retry verse"}));
  await screen.findByText("Restored verse text");
  expect(read).not.toHaveBeenCalled();
});
