// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
const ipc = vi.hoisted(() => ({invoke: vi.fn()}));
vi.mock("@tauri-apps/api/core", () => ({invoke:ipc.invoke,isTauri:()=>true,Channel:class {onmessage = (_p: unknown) => {};}}));
vi.mock("./Sheet", () => ({Sheet:({open,children,footer}: {open:boolean;children:React.ReactNode;footer:React.ReactNode}) => open ? <section>{children}{footer}</section> : null}));
import { ResourceUpdates } from "./ResourceUpdates";
afterEach(() => {cleanup(); vi.clearAllMocks();});
const offer = {installed_sequence:8,available_sequence:10,update_available:true,download_bytes:146677702,temporary_bytes:964652998,manifest_sha256:"reviewed-hash",release_url:"https://github.com/DavidHoenisch/fontes-v2/releases/tag/data-v1-stable-10"};
it("checks metadata first, displays size, then downloads the reviewed release and reopens", async () => {
  const reload = vi.fn(async () => {});
  ipc.invoke.mockImplementation(async (cmd,args) => {
    if(cmd === "library_check_update") return offer;
    if(cmd === "library_update") args.progress.onmessage({message:"Checking your library…",completed:0,total:null});
  });
  render(<ResourceUpdates reload={reload}/>);
  fireEvent.click(screen.getByText("Free resources"));
  expect(ipc.invoke).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", {name:"Check for updates"}));
  await screen.findByText("Version 8 → 10");
  expect(ipc.invoke).toHaveBeenCalledExactlyOnceWith("library_check_update");
  fireEvent.click(screen.getByRole("button",{name:"Download update · 140 MB"}));
  await screen.findByText("Your updated library is ready.");
  expect(ipc.invoke).toHaveBeenCalledWith("library_update",expect.objectContaining({manifestSha256:"reviewed-hash"}));
  expect(reload).toHaveBeenCalledTimes(1);
});
it("keeps a failed update retryable without reopening or claiming success", async () => {
  const reload = vi.fn();
  ipc.invoke.mockImplementation(async cmd => {if(cmd === "library_check_update") return offer; throw Error("Connection lost");});
  render(<ResourceUpdates reload={reload}/>);
  fireEvent.click(screen.getByText("Free resources"));
  fireEvent.click(screen.getByRole("button",{name:"Check for updates"}));
  fireEvent.click(await screen.findByRole("button",{name:"Download update · 140 MB"}));
  await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Connection lost"));
  expect(reload).not.toHaveBeenCalled();
  expect(screen.getByRole("button",{name:"Check for updates"})).toBeTruthy();
});
