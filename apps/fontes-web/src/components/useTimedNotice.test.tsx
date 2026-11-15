// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { useTimedNotice } from "./useTimedNotice";
afterEach(()=>{cleanup();vi.useRealTimers();});
it("expires success feedback and restarts the timer for repeated actions",()=>{
 vi.useFakeTimers();const {result}=renderHook(()=>useTimedNotice());
 act(()=>result.current[1]("1 note organized"));
 act(()=>vi.advanceTimersByTime(3000));expect(result.current[0]).toBe("1 note organized");
 act(()=>result.current[1]("1 note organized"));
 act(()=>vi.advanceTimersByTime(1000));expect(result.current[0]).toBe("1 note organized");
 act(()=>vi.advanceTimersByTime(2500));expect(result.current[0]).toBe("");
});
it("allows immediate dismissal and cleans up on unmount",()=>{
 vi.useFakeTimers();const {result,unmount}=renderHook(()=>useTimedNotice());
 act(()=>result.current[1]("Folder saved"));act(()=>result.current[1](""));expect(result.current[0]).toBe("");
 act(()=>result.current[1]("Folder saved"));unmount();expect(vi.getTimerCount()).toBe(0);
});
