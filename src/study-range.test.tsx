// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import i18n from "./i18n";
import {
  displayCards,
  quizCells,
  RangeDrill,
  sampleHands,
  scoreBand,
  scoreFrequencies,
} from "./components/StudyRange";
import type { Request } from "./types";

afterEach(async () => {
  cleanup();
  vi.unstubAllGlobals();
  await i18n.changeLanguage("zh-TW");
});

const cells = [
  { hand: "AA", strategy: { fold: 0.5, call: 0.5, raise: 0 }, issue: null, observed: null },
  { hand: "KK", strategy: { fold: 0, call: 1, raise: 0 }, issue: null, observed: null },
  { hand: "72o", strategy: null, issue: null, observed: null },
];
const matrix = {
  node: {
    id: "n1",
    hero: "BB",
    path: "RF",
    actions: [
      { code: "fold", label: "Fold", kind: "fold", size_bb: null },
      { code: "call", label: "Call", kind: "call", size_bb: null },
      { code: "raise", label: "Raise", kind: "raise", size_bb: 12 },
    ],
    ready_cells: 2,
    reference_cells: 0,
    source_url: "",
    frequencies: { AA: { fold: 0.5, call: 0.5, raise: 0 } },
    issues: {},
    reach: {},
  },
  source: {},
  cells,
  reasons: {},
  coverage: { total: 1, indexed: 1, complete: true, version: "hero-index/3" },
};

it("scores mixed strategies by total variation distance", () => {
  expect(scoreFrequencies({ a: 50, b: 50 }, { a: 50, b: 50 })).toEqual({
    tvd: 0,
    score: 100,
  });
  expect(scoreFrequencies({ a: 100 }, { a: 50, b: 50 })).toEqual({
    tvd: 50,
    score: 50,
  });
  expect(scoreFrequencies({ a: 25, b: 75 }, { a: 75, b: 25 })).toEqual({
    tvd: 50,
    score: 50,
  });
});

it("samples only drillable cells deterministically from a seed", () => {
  expect(quizCells(cells).map((c) => c.hand)).toEqual(["AA", "KK"]);
  const first = sampleHands(cells, 10, 42);
  const second = sampleHands(cells, 10, 42);
  expect(first).toEqual(second);
  expect(first.length).toBe(2);
  expect(sampleHands(cells, 1, 42).length).toBe(1);
});

it("maps suited and offsuit combos to display cards", () => {
  expect(displayCards("AKs")).toEqual(["As", "Ks"]);
  expect(displayCards("AKo")).toEqual(["As", "Kh"]);
});

it("bands scores", () => {
  expect(scoreBand(100)).toBe("excellent");
  expect(scoreBand(90)).toBe("good");
  expect(scoreBand(60)).toBe("needsWork");
});

it("drills a combo and reveals theory only after submitting", async () => {
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
  render(
    <RangeDrill
      pack="p1"
      node="n1"
      revision={0}
      refresh={vi.fn()}
      notify={vi.fn()}
      goTrainer={vi.fn()}
    />,
  );
  const requestOp = (r: { body: Request }) =>
    (r.body as unknown as { request?: { op?: string } }).request?.op;
  const matrixIndex = requests.findIndex(
    (r) => requestOp(r) === "matrix",
  );
  const itemsIndex = requests.findIndex(
    (r) => requestOp(r) === "items",
  );
  await respond(matrixIndex, matrix);
  await respond(itemsIndex, []);
  await waitFor(() =>
    expect(
      (screen.getByRole("button", { name: "開始操練" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
  fireEvent.click(screen.getByRole("button", { name: "開始操練" }));
  const hand = screen.getByText(/^(AA|KK)$/).textContent;
  const answer = hand === "AA" ? { Fold: "50", Call: "50" } : { Fold: "0", Call: "100" };
  expect(screen.queryByText("策略包頻率")).toBeNull();
  fireEvent.change(screen.getByLabelText("Fold %"), {
    target: { value: answer.Fold },
  });
  fireEvent.change(screen.getByLabelText("Call %"), {
    target: { value: answer.Call },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "提交答案" }),
  );
  await waitFor(() =>
    expect(screen.getByText("策略包頻率")).toBeTruthy(),
  );
  expect(screen.getByText("相似度 100.0：掌握良好。")).toBeTruthy();
});
