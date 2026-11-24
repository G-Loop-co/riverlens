// @vitest-environment jsdom
import { StrictMode } from "react";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DownloadEvent } from "@tauri-apps/plugin-updater";
import { AUTO_UPDATE_KEY, useAppUpdates } from "./components/AppUpdates";

const native = vi.hoisted(() => ({
  desktop: true,
  check: vi.fn(),
  relaunch: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: native.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: native.relaunch }));
vi.mock("./api", () => ({
  get desktop() {
    return native.desktop;
  },
  errorText: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
}));

function pending() {
  return {
    version: "0.8.0",
    body: "Release notes",
    download: vi.fn(async (_progress: (event: DownloadEvent) => void) => {}),
    install: vi.fn(async () => {}),
    close: vi.fn(async () => {}),
  };
}

function untilResolved() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

beforeEach(() => {
  vi.resetAllMocks();
  native.desktop = true;
  const values = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  });
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("desktop release updates", () => {
  it("checks once at startup and releases a deferred update without downloading it", async () => {
    const update = pending();
    native.check.mockResolvedValue(update);
    const { result } = renderHook(() => useAppUpdates(() => null), {
      wrapper: StrictMode,
    });
    await waitFor(() => expect(result.current.phase).toBe("available"));
    expect(native.check).toHaveBeenCalledTimes(1);
    expect(result.current.notice).toBe(true);
    act(() => result.current.later());
    expect(result.current.notice).toBe(false);
    expect(update.close).toHaveBeenCalledTimes(1);
    expect(update.download).not.toHaveBeenCalled();
    await act(() => result.current.checkNow());
    expect(result.current.phase).toBe("available");
  });

  it("honors the saved switch; manual offline checks can retry, and the web preview stays offline", async () => {
    localStorage.setItem(AUTO_UPDATE_KEY, "false");
    native.check
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(null);
    const { result } = renderHook(() => useAppUpdates(() => null));
    expect(result.current.automatic).toBe(false);
    expect(native.check).not.toHaveBeenCalled();
    await act(() => result.current.checkNow());
    expect(result.current.error).toBe("offline");
    expect(result.current.notice).toBe(false);
    await act(() => result.current.checkNow());
    expect(result.current.phase).toBe("current");
    native.desktop = false;
    act(() => result.current.changeAutomatic(true));
    await act(() => result.current.checkNow());
    expect(native.check).toHaveBeenCalledTimes(2);
    expect(localStorage.getItem(AUTO_UPDATE_KEY)).toBe("true");
  });

  it("checks again before installation and reuses the signed download when work finishes", async () => {
    const update = pending(),
      transfer = untilResolved();
    native.check.mockResolvedValue(update);
    update.download.mockImplementation(async (progress) => {
      progress({ event: "Started", data: { contentLength: 100 } });
      progress({ event: "Progress", data: { chunkLength: 30 } });
      await transfer.promise;
    });
    const { result, rerender } = renderHook(
      ({ blocker }: { blocker: string | null }) => useAppUpdates(() => blocker),
      { initialProps: { blocker: null as string | null } },
    );
    await waitFor(() => expect(result.current.phase).toBe("available"));
    let installing!: Promise<void>;
    await act(async () => {
      installing = result.current.install();
    });
    expect(result.current.percent).toBe(30);
    rerender({ blocker: "Save the note first" });
    await act(async () => {
      transfer.resolve();
      await installing;
    });
    expect(result.current.error).toBe("Save the note first");
    expect(result.current.phase).toBe("available");
    expect(update.install).not.toHaveBeenCalled();
    rerender({ blocker: null });
    await act(() => result.current.install());
    expect(update.download).toHaveBeenCalledTimes(1);
    expect(update.install).toHaveBeenCalledTimes(1);
    expect(native.relaunch).toHaveBeenCalledTimes(1);
  });

  it("blocks active work before download and rejects failed signature verification", async () => {
    const update = pending();
    native.check.mockResolvedValue(update);
    update.download.mockRejectedValueOnce(
      new Error("Signature verification failed"),
    );
    const getBlocker = vi
      .fn<() => string | null>()
      .mockReturnValueOnce("Import active")
      .mockReturnValue(null);
    const { result } = renderHook(() => useAppUpdates(getBlocker));
    await waitFor(() => expect(result.current.phase).toBe("available"));
    await act(() => result.current.install());
    expect(update.download).not.toHaveBeenCalled();
    await act(() => result.current.install());
    expect(result.current.error).toMatch(/signature/i);
    expect(result.current.phase).toBe("idle");
    expect(update.close).toHaveBeenCalledTimes(1);
    expect(update.install).not.toHaveBeenCalled();
    expect(native.relaunch).not.toHaveBeenCalled();
  });

  it("retries only restart when the new package is already installed", async () => {
    const update = pending();
    native.check.mockResolvedValue(update);
    native.relaunch.mockRejectedValueOnce(new Error("Restart failed"));
    const { result } = renderHook(() => useAppUpdates(() => null));
    await waitFor(() => expect(result.current.phase).toBe("available"));
    await act(() => result.current.install());
    expect(result.current.phase).toBe("restart");
    act(() => result.current.later());
    expect(result.current.notice).toBe(false);
    await act(() => result.current.install());
    expect(update.download).toHaveBeenCalledTimes(1);
    expect(update.install).toHaveBeenCalledTimes(1);
    expect(native.relaunch).toHaveBeenCalledTimes(2);
  });

  it("never starts installation when the controller unmounts during download", async () => {
    const update = pending(),
      transfer = untilResolved();
    native.check.mockResolvedValue(update);
    update.download.mockImplementation(() => transfer.promise);
    const { result, unmount } = renderHook(() => useAppUpdates(() => null));
    await waitFor(() => expect(result.current.phase).toBe("available"));
    let installing!: Promise<void>;
    await act(async () => {
      installing = result.current.install();
    });
    unmount();
    transfer.resolve();
    await installing;
    expect(update.install).not.toHaveBeenCalled();
    expect(update.close).toHaveBeenCalledTimes(1);
  });
});
