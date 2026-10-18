// @vitest-environment jsdom
import {afterEach,expect,it,vi} from "vitest";
import {cleanup,fireEvent,render,screen} from "@testing-library/react";
import {OccurrenceExplorer} from "./OccurrenceExplorer";
afterEach(cleanup);
it("reveals references inside their book, then fetches verse text only on expansion",async()=>{
  const page=vi.fn(async()=>({verses:[{ref_id:1001001,label:"Genesis 1:1"}],has_more:false}));
  const verse=vi.fn(async()=>"In the beginning God created the heaven and the earth.");
  render(<OccurrenceExplorer books={[{id:1,name:"Genesis",verses:141}]} loadPage={page} loadVerse={verse}/>);
  expect(page).not.toHaveBeenCalled();expect(verse).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button",{name:"Genesis 141 verses"}));
  const reference=await screen.findByRole("button",{name:"Genesis 1:1"});
  expect(page).toHaveBeenCalledExactlyOnceWith(1,0);expect(verse).not.toHaveBeenCalled();
  expect(reference.closest(".occurrence-book-section")?.textContent).toContain("Genesis 141 verses");
  fireEvent.click(reference);
  await screen.findByText("In the beginning God created the heaven and the earth.");
  expect(verse).toHaveBeenCalledExactlyOnceWith(1001001);
  fireEvent.click(reference);expect(screen.queryByText("In the beginning God created the heaven and the earth.")).toBeNull();
  fireEvent.click(reference);expect(verse).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button",{name:"Genesis 141 verses"}));
  expect(screen.queryByRole("button",{name:"Genesis 1:1"})).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Genesis 141 verses"}));
  expect(page).toHaveBeenCalledTimes(1);
});
it("appends more reference headings in place without expanding verse bodies",async()=>{
  const page=vi.fn(async(_book:number,offset:number)=>({verses:[{ref_id:1001001+offset,label:`Genesis 1:${1+offset}`}],has_more:offset===0}));
  const verse=vi.fn(async()=>"Text");
  render(<OccurrenceExplorer books={[{id:1,name:"Genesis",verses:2}]} loadPage={page} loadVerse={verse}/>);
  fireEvent.click(screen.getByRole("button",{name:"Genesis 2 verses"}));
  fireEvent.click(await screen.findByRole("button",{name:"Show more references in Genesis"}));
  await screen.findByRole("button",{name:"Genesis 1:2"});
  expect(screen.getByRole("button",{name:"Genesis 1:1"})).toBeTruthy();
  expect(page).toHaveBeenLastCalledWith(1,1);expect(verse).not.toHaveBeenCalled();
});
it("allows retrying a failed reference request in the expanded book",async()=>{
  const page=vi.fn().mockRejectedValueOnce(Error("Offline")).mockResolvedValueOnce({verses:[{ref_id:1001001,label:"Genesis 1:1"}],has_more:false});
  render(<OccurrenceExplorer books={[{id:1,name:"Genesis",verses:1}]} loadPage={page} loadVerse={async()=>"Text"}/>);
  fireEvent.click(screen.getByRole("button",{name:"Genesis 1 verses"}));
  fireEvent.click(await screen.findByRole("button",{name:"Retry references"}));
  await screen.findByRole("button",{name:"Genesis 1:1"});
  expect(screen.queryByRole("alert")).toBeNull();
});
