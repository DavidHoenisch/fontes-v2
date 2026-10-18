// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
const bridge = vi.hoisted(() => ({
  invoke: vi.fn(),
  show: vi.fn(async () => {}),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: bridge.invoke,
  isTauri: () => true,
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ show: bridge.show }),
}));
vi.mock("./App", () => ({ default: () => <main>Reader ready</main> }));
import { NativeApp } from "./native";
afterEach(cleanup);
beforeEach(() => {
  bridge.invoke.mockReset();
  bridge.show.mockClear();
});
describe("native cold start", () => {
  it("never flashes setup or database-loading text while an installed library opens", async () => {
    let resolve!: (value: unknown) => void;
    bridge.invoke.mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }),
    );
    const { container } = render(<NativeApp />);
    expect(container.querySelector(".native-startup")).not.toBeNull();
    expect(screen.queryByRole("button")).toBeNull();
    expect(container.textContent).toBe("");
    expect(bridge.show).not.toHaveBeenCalled();
    await act(async () => resolve({ state: {}, data: {} }));
    expect(screen.getByText("Reader ready")).toBeTruthy();
    expect(screen.queryByText("Install free resources")).toBeNull();
    await waitFor(() => expect(bridge.show).toHaveBeenCalled());
  });
  it("offers one full free-resource install only after confirming first run", async () => {
    bridge.invoke.mockRejectedValueOnce({
      kind: "missing",
      message: "No library installed.",
    });
    render(<NativeApp />);
    const install = await screen.findByRole("button", {
      name: "Install free resources",
    });
    expect(screen.queryByRole("button", { name: /reading|study/i })).toBeNull();
    let finish!: (value: unknown) => void;
    bridge.invoke.mockReturnValueOnce(
      new Promise((r) => {
        finish = r;
      }),
    );
    fireEvent.click(install);
    await waitFor(() =>
      expect(bridge.invoke).toHaveBeenLastCalledWith("library_install"),
    );
    expect(await screen.findByRole("status")).toBeTruthy();
    bridge.invoke.mockResolvedValueOnce({ state: {}, data: {} });
    await act(async () => finish(undefined));
    expect(await screen.findByText("Reader ready")).toBeTruthy();
  });
  it("keeps a real open failure visible without offering to overwrite the library", async () => {
    bridge.invoke.mockRejectedValueOnce({
      kind: "failed",
      message: "Database validation failed.",
    });
    render(<NativeApp />);
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(
      screen.queryByRole("button", { name: "Install free resources" }),
    ).toBeNull();
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
  });
});
