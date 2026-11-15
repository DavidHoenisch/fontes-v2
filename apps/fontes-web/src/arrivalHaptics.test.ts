// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({isTauri:vi.fn(),invoke:vi.fn()}));
import { isTauri, invoke } from "@tauri-apps/api/core";
import { arrivalHaptic } from "./arrivalHaptics";
afterEach(() => {vi.resetAllMocks();vi.restoreAllMocks();});
it("does not request native feedback in a browser", async () => {
  vi.mocked(isTauri).mockReturnValue(false);
  await arrivalHaptic();
  expect(invoke).not.toHaveBeenCalled();
});
it("requests one fixed native cue in the visible app", async () => {
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(invoke).mockResolvedValue(undefined);
  await arrivalHaptic();
  expect(invoke).toHaveBeenCalledExactlyOnceWith("verse_arrival_haptic");
});
it("skips background feedback and treats unavailable hardware as optional", async () => {
  vi.mocked(isTauri).mockReturnValue(true);
  const visibility=vi.spyOn(document,"visibilityState","get").mockReturnValue("hidden");
  await arrivalHaptic();
  expect(invoke).not.toHaveBeenCalled();
  visibility.mockReturnValue("visible");
  vi.mocked(invoke).mockRejectedValue(Error("Unavailable"));
  await expect(arrivalHaptic()).resolves.toBeUndefined();
});
