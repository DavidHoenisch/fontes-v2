// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ResourceCitations } from "./ResourceCitations";
import type { Live } from "../native";
afterEach(cleanup);
it("deduplicates article citations and previews their text without navigation", async () => {
  const send = vi.fn(async () => ({data:{verses:[{ref_id:43003016,label:"John 3:16",text:"For God so loved the world"}]}}));
  const live = {state:{books:[{id:43,name:"John"}]},send} as unknown as Live;
  const citation = {start_ref:43003016,end_ref:43003016,basis:"citation"};
  render(<ResourceCitations live={live} links={[citation,citation]}/>);
  expect(screen.getAllByRole("button",{name:"John 3:16"})).toHaveLength(1);
  fireEvent.click(screen.getByRole("button",{name:"John 3:16"}));
  await waitFor(() => expect(screen.getByText("For God so loved the world")).toBeTruthy());
  expect(send).toHaveBeenCalledExactlyOnceWith({command:"reading.preview",params:{start_ref:43003016,end_ref:43003016}});
});
