// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { HashRouter } from "react-router";
import { afterEach, expect, it, vi } from "vitest";
import "./i18n";
import BluffReport from "./components/BluffReport";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
});

it("uses the same size and class for summary, hand evidence and both replay steps", async () => {
  const money = {
    segment_bb: -1,
    net_bb: 3.5,
    segment: "-0.02",
    net: "0.07",
    currency: "USD",
  };
  const summary = {
    coverage: { total: 1, indexed: 1, complete: true },
    unknown: 0,
    exclusions: {},
    stakes: ["0.01/0.02"],
    currencies: ["USD"],
    groups: [
      {
        ...money,
        category: "flop_draw",
        bucket: "medium",
        attempts: 1,
        hands: 1,
        folded: 0,
        settled: 1,
      },
    ],
  };
  const detail = {
    rows: [
      {
        ...money,
        id: 7,
        hand_id: "SYN7",
        seq: 8,
        start: 8,
        street: "flop",
        holding: "draw",
        outs: 9,
        action: "bet",
        response: "raised",
        size: 50,
        date: "2026-09-01",
        followups: [{ seq: 10, action: "raise", size: 50 }],
      },
    ],
    next: null,
  };
  const requests: unknown[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (_url, opts) => {
      const { query } = JSON.parse(opts.body);
      requests.push(query);
      return {
        ok: true,
        json: async () => ({ result: query.cell ? detail : summary }),
      };
    }),
  );
  const open = vi.fn();
  window.history.replaceState(
    null,
    "",
    "#/hands/bluffs?b_breakdown=size&b_players=multi&b_currency=all",
  );
  render(
    <HashRouter>
      <BluffReport revision={0} profiles={[]} openHand={open} health={null} />
    </HashRouter>,
  );
  const group = await screen.findByRole("button", { name: "翻牌 · 強聽牌" });
  expect(within(group.closest("tr")!).getByText("0%")).toBeTruthy();
  fireEvent.click(group);
  fireEvent.click(await screen.findByRole("button", { name: "SYN7" }));
  expect(open).toHaveBeenLastCalledWith(7, 8);
  fireEvent.click(screen.getByRole("button", { name: /同街再加注/ }));
  expect(open).toHaveBeenLastCalledWith(7, 10);
  expect(requests).toContainEqual(
    expect.objectContaining({
      view: "single",
      breakdown: "size",
      players: "multi",
      cell: { category: "flop_draw", bucket: "medium" },
    }),
  );
  expect(window.location.hash).toContain("b_players=multi");
});
