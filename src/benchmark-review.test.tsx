// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import i18n from "./i18n";
import {
  assess,
  cohorts,
  priorities,
  ranges,
  type Counts,
  type Metric,
  type PositionSummary,
  type ReviewSummary,
} from "./benchmarks";
import { BenchmarkReview, LastSession } from "./components/BenchmarkReview";

function seat(
  overrides: Partial<Omit<PositionSummary, "metrics">> = {},
  metrics: Partial<Record<Metric, Counts>> = {},
): PositionSummary {
  return {
    currency: "USD",
    game: "Rush",
    stakes: "0.01/0.02",
    hands: 100,
    position: "BB",
    ...overrides,
    metrics: {
      vpip: [24, 100],
      pfr: [20, 100],
      three_bet: [8, 100],
      fold_three_bet: [55, 100],
      cbet: [60, 100],
      fold_cbet: [50, 100],
      ...metrics,
    },
  };
}
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("six-handed cash benchmarks", () => {
  it.each([
    ["vpip", 21, 27],
    ["pfr", 17, 23],
    ["three_bet", 6, 10],
    ["fold_three_bet", 45, 65],
    ["cbet", 50, 70],
    ["fold_cbet", 40, 55],
  ] as const)(
    "uses an inclusive %s teaching band and leaves zero opportunities unscored",
    (id, low, high) => {
      expect(ranges[id]).toEqual([low, high]);
      expect(assess(id, [0, 0])).toEqual({
        value: null,
        gap: 0,
        priority: false,
      });
      for (const edge of [low, high]) {
        expect(assess(id, [edge, 100])).toEqual({
          value: edge,
          gap: 0,
          priority: false,
        });
      }
    },
  );

  it("requires 100 opportunities and a Wilson interval wholly outside the band", () => {
    expect(assess("three_bet", [0, 99]).priority).toBe(false);
    expect(assess("three_bet", [0, 100]).priority).toBe(true);
    expect(assess("three_bet", [13, 100]).priority).toBe(false);
    expect(assess("three_bet", [16, 100]).priority).toBe(true);
    expect(assess("three_bet", [60, 400])).toEqual({
      value: 15,
      gap: 5,
      priority: true,
    });
  });

  it("pools counts across seats, retaining separate currencies, stakes and table formats", () => {
    const rows = [
      seat({ position: "BB", hands: 200 }, { three_bet: [10, 50] }),
      seat({ position: "BTN", hands: 100 }, { three_bet: [1, 100] }),
      seat({ game: "Cash" }),
      seat({ stakes: "0.02/0.05" }),
      seat({ currency: "EUR" }),
    ];
    const grouped = cohorts(rows);
    expect(grouped).toHaveLength(4);
    expect(grouped[0]).toMatchObject({
      currency: "USD",
      game: "Rush",
      stakes: "0.01/0.02",
      hands: 300,
      metrics: { three_bet: [11, 150] },
    });
    expect(grouped[0].positions.map((p) => p.position)).toEqual(["BB", "BTN"]);
    expect(rows[0].metrics.three_bet).toEqual([10, 50]);
  });

  it("selects at most three eligible gaps and suppresses overall rankings for constrained spots", () => {
    const grouped = cohorts([
      seat(
        {},
        {
          pfr: [900, 1000],
          vpip: [900, 1000],
          three_bet: [600, 1000],
          fold_three_bet: [0, 1000],
          cbet: [300, 1000],
          fold_cbet: [300, 1000],
        },
      ),
    ]);
    const top = priorities(grouped, {
      game: "Rush",
      currency: "USD",
      player_count: 6,
    });
    expect(top.map((r) => r.id)).toEqual(["pfr", "vpip", "three_bet"]);
    for (const filter of [
      { position: "BB" },
      { effective_min: 90 },
      { flop_players: 2 },
      { player_count: 5 },
      { tag: "study" },
    ]) {
      expect(priorities(grouped, filter)).toEqual([]);
    }
  });
});

describe("review navigation", () => {
  it("opens the exact latest session in its own currency", () => {
    const drill = vi.fn();
    render(
      <LastSession
        latest={{
          session: "sample:session.txt",
          currency: "EUR",
          time: 1770000000,
          hands: 8,
          net: "2.75",
          net_bb: 137.5,
        }}
        drill={drill}
      />,
    );
    const region = screen.getByRole("heading", {
      name: i18n.t("review.last"),
    }).parentElement!;
    expect(region.textContent).toMatch(/EUR.*2[.]75/);
    fireEvent.click(
      screen.getByRole("button", { name: i18n.t("review.openSession") }),
    );
    expect(drill).toHaveBeenCalledWith({
      session: "sample:session.txt",
      currency: "EUR",
    });
  });

  it("drills actions, declined opportunities and the selected seat with their cohort", () => {
    const drill = vi.fn();
    const data: ReviewSummary = {
      positions: [
        seat(),
        seat({ position: "BTN" }),
        seat({ game: "Cash", hands: 20 }),
      ],
      latest: null,
      has_hands: true,
    };
    render(
      <BenchmarkReview
        data={data}
        filter={{}}
        revision={0}
        open={vi.fn()}
        drill={drill}
      />,
    );
    const card = screen
      .getByText(i18n.t("bench.three_bet"), { selector: "strong" })
      .closest("button")!;
    fireEvent.click(card);
    const base = {
      game: "Rush",
      stakes: "0.01/0.02",
      currency: "USD",
      player_count: 6,
      stat: "three_bet",
    };
    for (const [label, mode] of [
      ["review.hits", "numerator"],
      ["review.missed", "benchmark_missed"],
      ["review.allOpp", "benchmark_opportunities"],
    ]) {
      fireEvent.click(screen.getByRole("button", { name: i18n.t(label) }));
      expect(drill).toHaveBeenLastCalledWith(
        expect.objectContaining({ ...base, stat_mode: mode }),
      );
    }
    fireEvent.click(screen.getByRole("button", { name: /^BTN / }));
    expect(drill).toHaveBeenLastCalledWith({
      ...base,
      position: "BTN",
      stat_mode: "benchmark_opportunities",
    });
    fireEvent.change(screen.getByRole("combobox"), {
      target: { value: JSON.stringify(["USD", "Cash", "0.01/0.02"]) },
    });
    fireEvent.click(
      screen
        .getByText(i18n.t("bench.three_bet"), { selector: "strong" })
        .closest("button")!,
    );
    fireEvent.click(
      screen.getByRole("button", { name: i18n.t("review.missed") }),
    );
    expect(drill).toHaveBeenLastCalledWith(
      expect.objectContaining({ game: "Cash", stat_mode: "benchmark_missed" }),
    );
  });

  it("shows the published BB versus BTN model for its 2.5bb open only and replays the exact decision", async () => {
    const open = vi.fn();
    const requests: unknown[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (_url: string, init: RequestInit) => {
        requests.push(JSON.parse(String(init.body)));
        return {
          ok: true,
          json: async () => ({
            result: {
              n: 12,
              raises: 2,
              calls: 5,
              rows: [
                {
                  id: 9,
                  seq: 7,
                  cards: "A5s",
                  action: "call",
                  hand_id: "SYN9",
                },
              ],
            },
          }),
        };
      }),
    );
    const data: ReviewSummary = {
      positions: [seat()],
      latest: null,
      has_hands: true,
    };
    const ui = (position?: string) => (
      <BenchmarkReview
        data={data}
        filter={{ position }}
        revision={3}
        drill={vi.fn()}
        open={open}
      />
    );
    const view = render(ui());
    fireEvent.click(
      screen
        .getByText(i18n.t("bench.three_bet"), { selector: "strong" })
        .closest("button")!,
    );
    fireEvent.click(
      screen.getByRole("button", { name: i18n.t("review.blindSpot") }),
    );
    expect(
      screen.getByRole("link", { name: /SolvePoker/ }).getAttribute("href"),
    ).toBe("https://solvepoker.app/nlhe-stack-depth");
    fireEvent.click(await screen.findByRole("button", { name: /SYN9/ }));
    expect(open).toHaveBeenLastCalledWith(9, 7);
    expect(requests).toContainEqual({
      op: "review_blind_spot",
      open_bb: "2.5",
      filter: {
        currency: "USD",
        game: "Rush",
        stakes: "0.01/0.02",
        position: "BB",
      },
    });
    fireEvent.change(
      screen.getByRole("combobox", {
        name: new RegExp(i18n.t("review.openSize")),
      }),
      { target: { value: "2" } },
    );
    expect(screen.queryByRole("link", { name: /SolvePoker/ })).toBeNull();
    expect(screen.getByText(i18n.t("review.noSize"))).toBeTruthy();
    await screen.findByRole("button", { name: /SYN9/ });
    expect(requests).toContainEqual(expect.objectContaining({ open_bb: "2" }));
    view.rerender(ui("BTN"));
    expect(
      screen.queryByRole("button", { name: i18n.t("review.blindSpot") }),
    ).toBeNull();
  });
});
