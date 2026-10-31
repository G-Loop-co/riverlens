// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { composeCoachQuestion, MdaWizard } from "./components/MdaWizard";
import type { Request } from "./types";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const stat = {
  metric: "blind_fold",
  bb_units: 2_000_000,
  street: "preflop",
  context: "BB",
  numerator: 140,
  opportunities: 250,
  pct: 56,
  wilson95: [50, 62] as [number, number],
  period_interval_overlap: true,
  showdown_coverage_pct: null,
  periods: [],
};
const catalog = {
  dataset: {
    id: "ds1",
    name: "GG",
    pool: "GG Regular",
    timezone: "UTC",
    cloud_enabled: true,
    mcp_enabled: false,
  },
  version: "v1",
  hands: 1000,
  quarantined: 0,
  stakes: [
    { bb_units: 2_000_000, hands: 1000, date_min: 1, date_max: 2 },
  ],
  metrics: [
    { key: "blind_fold", definition: "盲位 fold 決策" },
    { key: "cbet", definition: "C-bet 機會" },
  ],
};

it("composes a coach question with counts, interval and periods", () => {
  const question = composeCoachQuestion("BB 棄牌過多", stat);
  expect(question).toContain("研究假設：BB 棄牌過多");
  expect(question).toContain("140/250（56.0%）");
  expect(question).toContain("Wilson 95% 50.0–62.0%");
  expect(question).toContain("前後期區間重疊");
  expect(composeCoachQuestion("  ", stat)).toContain(
    "研究假設：（未填寫，請先依數據提出可檢驗的假設）",
  );
});

it("walks through hypothesis, stats, hand verification and hands off to coach", async () => {
  const requests: { body: Request; resolve: (v: Response) => void }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(
      (_url, options) =>
        new Promise<Response>((resolve) =>
          requests.push({ body: JSON.parse(options.body), resolve }),
        ),
    ),
  );
  const respond = (index: number, result: unknown) =>
    act(async () =>
      requests[index].resolve({
        ok: true,
        json: async () => ({ result }),
      } as Response),
    );
  const goCoach = vi.fn();
  render(
    <MdaWizard
      catalog={catalog}
      datasetId="ds1"
      startReplay={vi.fn()}
      goCoach={goCoach}
    />,
  );
  const name = (
    r: { body: Request },
  ) => (r.body as unknown as { request?: { name?: string } }).request?.name;
  const statsIndex = () => requests.findIndex((r) => name(r) === "query_mda_stats");
  const spotsIndex = () => requests.findIndex((r) => name(r) === "find_mda_spots");
  fireEvent.change(screen.getByLabelText("常見研究假設"), {
    target: { value: "BB 面對 open 是否棄牌過多" },
  });
  expect(
    (screen.getByLabelText("指標") as HTMLSelectElement).value,
  ).toBe("blind_fold");
  fireEvent.click(screen.getByRole("button", { name: "下一步" }));
  fireEvent.click(screen.getByRole("button", { name: "下一步" }));
  await respond(statsIndex(), {
    version: "v1",
    data: { rows: [stat], next_cursor: null },
  });
  expect(screen.getByText("140 / 250")).toBeTruthy();
  expect(screen.getByText(/56\.0%（50\.0–62\.0%）/)).toBeTruthy();
  expect(screen.queryByText(/低於 200/)).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "下一步" }));
  await respond(spotsIndex(), {
    version: "v1",
    data: {
      rows: [
        {
          hand_key: "abcdef1234567890",
          seq: 2,
          seat: 1,
          day: 1,
          position: "BB",
          opponent: "BTN",
          street: "preflop",
          pot_type: "SRP",
          players: 6,
          board: "",
          action: "fold",
          effective_bb: 100,
          facing_bet_pct: 200,
        },
      ],
      next_cursor: null,
    },
  });
  expect(screen.getByText("abcdef12 · 2")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "下一步" }));
  fireEvent.click(screen.getByRole("button", { name: "帶到 AI 剝削分析" }));
  expect(goCoach).toHaveBeenCalledWith(
    expect.stringContaining("研究假設：BB 面對 open 是否棄牌過多"),
  );
});
