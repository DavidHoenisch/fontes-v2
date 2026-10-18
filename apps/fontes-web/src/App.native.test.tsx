// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
vi.mock("./data/preview", () => ({
  books:[{id:1,name:"Genesis",art:"/cover.svg",title:"Beginnings",paragraphs:["A book introduction"]}],
  passages:[{book:1,chapter:1,verses:["In the beginning"],section:"Beginning"}],
  samples:[],loadNotes:()=>[],persistNotes:()=>{},
}));
import App from "./App";
import type { Live } from "./native";
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
