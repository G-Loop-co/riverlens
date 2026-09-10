// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useRpc } from "./api";
import App from "./App";
import { HandsView } from "./components/HandsView";
import { Field } from "./components/UI";
import { readFilter, writePreference } from "./preferences";
import type { HandPage, HandRow, Request } from "./types";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function delayedFetch() {
  const requests: {
    body: Request;
    resolve: (response: Response) => void;
    reject: (error: Error) => void;
  }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(
      (_url, options) =>
        new Promise<Response>((resolve, reject) => {
          requests.push({ body: JSON.parse(options.body), resolve, reject });
        }),
    ),
  );
  return requests;
}
function respond(
  request: ReturnType<typeof delayedFetch>[number],
  result: unknown,
) {
  request.resolve({ ok: true, json: async () => ({ result }) } as Response);
}
const row = (id: number, position: string): HandRow => ({
  id,
  hand_id: `SYN${id}`,
  played_at: 1788192000 + id,
  date: "2026-09-01",
  position,
  hand_class: "AA",
  net: "0.12",
  net_bb: 6,
  stakes: "0.01/0.02",
  pot_type: "SRP",
  game: "Cash",
  status: "valid",
  ev_status: "not_applicable",
  reviewed: false,
  sort_value: id,
});

describe("UI request and storage regressions", () => {
  it("keeps field help separate from its accessible name", () => {
    render(
      <Field label="標籤" help="以逗號分隔">
        <input />
      </Field>,
    );
    const input = screen.getByRole("textbox", { name: "標籤" });
    expect(
      document.getElementById(input.getAttribute("aria-describedby")!)
        ?.textContent,
    ).toBe("以逗號分隔");
  });
  it("hides the previous query immediately and ignores a late response", async () => {
    const requests = delayedFetch();
    const { result, rerender } = renderHook(
      ({ position }) =>
        useRpc<{ hands: number }>({ op: "overview", filter: { position } }),
      { initialProps: { position: "BTN" } },
    );
    await act(async () => respond(requests[0], { hands: 10 }));
    expect(result.current.data?.hands).toBe(10);
    rerender({ position: "SB" });
    expect(result.current.data).toBeNull();
    expect(result.current.loading).toBe(true);
    rerender({ position: "BB" });
    await act(async () => respond(requests[2], { hands: 20 }));
    await act(async () => respond(requests[1], { hands: 99 }));
    expect(result.current.data?.hands).toBe(20);
  });

  it("removes cached results after a refresh error", async () => {
    const requests = delayedFetch();
    const { result, rerender } = renderHook(
      ({ revision }) =>
        useRpc<{ hands: number }>({ op: "overview", filter: {} }, revision),
      { initialProps: { revision: 0 } },
    );
    await act(async () => respond(requests[0], { hands: 10 }));
    rerender({ revision: 1 });
    await act(async () => requests[1].reject(new Error("engine unavailable")));
    expect(result.current.error).toBe("engine unavailable");
    expect(result.current.data).toBeNull();
    expect(result.current.loading).toBe(false);
  });

  it("does not append an old page after the user switches filters", async () => {
    const requests = delayedFetch();
    const props = { revision: 0, open: vi.fn(), notify: vi.fn() };
    const { rerender } = render(
      <HandsView {...props} filter={{ position: "BTN" }} />,
    );
    await act(async () =>
      respond(requests[0], {
        rows: [row(1, "BTN")],
        next_cursor: { id: 1, value: 1 },
      } satisfies HandPage),
    );
    fireEvent.click(screen.getByRole("button", { name: "載入更多" }));
    rerender(<HandsView {...props} filter={{ position: "BB" }} />);
    expect(screen.queryByText("#SYN1")).toBeNull();
    await act(async () =>
      respond(requests[2], { rows: [row(3, "BB")], next_cursor: null }),
    );
    await act(async () =>
      respond(requests[1], { rows: [row(2, "BTN")], next_cursor: null }),
    );
    expect(screen.queryByText("#SYN2")).toBeNull();
    expect(screen.getByText("#SYN3")).toBeTruthy();
  });

  it("keeps checkbox keyboard input separate from opening a hand and catches export dialog errors", async () => {
    const requests = delayedFetch();
    const open = vi.fn();
    render(<HandsView revision={0} open={open} notify={vi.fn()} filter={{}} />);
    await act(async () =>
      respond(requests[0], { rows: [row(1, "BTN")], next_cursor: null }),
    );
    fireEvent.keyDown(screen.getByRole("checkbox", { name: "選取 SYN1" }), {
      key: "Enter",
    });
    expect(open).not.toHaveBeenCalled();
    vi.spyOn(window, "prompt").mockImplementation(() => {
      throw new Error("dialog unavailable");
    });
    fireEvent.click(screen.getByRole("button", { name: "CSV" }));
    await waitFor(() =>
      expect(screen.getByRole("alert").textContent).toContain(
        "dialog unavailable",
      ),
    );
    expect(
      (screen.getByRole("button", { name: "CSV" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false);
  });

  it("recovers from invalid saved filters and storage being disabled", () => {
    let saved = "null";
    vi.stubGlobal("localStorage", {
      getItem: () => saved,
      setItem: () => {
        throw new Error("blocked");
      },
    });
    expect(readFilter()).toEqual({});
    saved =
      '{"position":"BTN","reviewed":false,"stack_min":-1,"unknown":"value"}';
    expect(readFilter()).toEqual({ position: "BTN", reviewed: false });
    expect(() => writePreference("riverlens-filter", "{}")).not.toThrow();
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("blocked");
      },
    });
    expect(readFilter()).toEqual({});
  });

  it("applies a database saved filter without null chips while retaining false and zero", async () => {
    const storage = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
    });
    const requests = delayedFetch();
    render(<App />);
    const savedRequest = requests.find(
      (request) => request.body.op === "saved_filters",
    )!;
    await act(async () =>
      respond(savedRequest, [
        {
          id: 1,
          name: "Audit BB",
          filter: {
            position: "BB",
            reviewed: false,
            stack_min: 0,
            game: null,
            session: null,
            tag: null,
            date_from: null,
          },
        },
      ]),
    );
    fireEvent.change(screen.getByRole("combobox", { name: "已儲存篩選" }), {
      target: { value: "1" },
    });
    expect(screen.getByRole("button", { name: "位置: BB" })).toBeTruthy();
    expect(screen.getByRole("button", { name: /進階篩選/ }).textContent).toBe(
      "進階篩選3",
    );
    expect(document.querySelector(".filter-chips")?.textContent).not.toContain(
      "null",
    );
    expect(JSON.parse(localStorage.getItem("riverlens-filter")!)).toEqual({
      position: "BB",
      reviewed: false,
      stack_min: 0,
    });
    fireEvent.click(screen.getByRole("button", { name: "位置: BB" }));
    expect(screen.queryByRole("button", { name: "位置: BB" })).toBeNull();
    expect(JSON.parse(localStorage.getItem("riverlens-filter")!)).toEqual({
      reviewed: false,
      stack_min: 0,
    });
  });
});
