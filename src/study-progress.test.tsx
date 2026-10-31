// @vitest-environment jsdom
import {
  act,
  cleanup,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import i18n from "./i18n";
import { leakStatus, StudyProgress } from "./components/StudyProgress";
import type { Request } from "./types";

afterEach(async () => {
  cleanup();
  vi.unstubAllGlobals();
  await i18n.changeLanguage("zh-TW");
});

const row = (over: Partial<Parameters<typeof leakStatus>[0]>) => ({
  name_key: null,
  id: "x",
  name: "BB fold vs open",
  source: "custom" as const,
  note: "",
  action: "fold",
  low: 50,
  high: 60,
  actual: 55,
  gap: null,
  interval: [50.1, 59.9] as [number, number],
  opportunities: 300,
  hits: 165,
  enough: true,
  priority: 5,
  ...over,
});

it("classifies leak rows by sample sufficiency and target range", () => {
  expect(leakStatus(row({}))).toBe("onTrack");
  expect(leakStatus(row({ actual: 70 }))).toBe("offTarget");
  expect(leakStatus(row({ enough: false }))).toBe("tooFew");
  expect(leakStatus(row({ actual: null }))).toBe("tooFew");
  expect(leakStatus(row({ low: null, high: null }))).toBe("tooFew");
});

it("renders trainer, leak and drill progress on one page", async () => {
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
    <StudyProgress
      filter={{}}
      pack=""
      revision={0}
      open={vi.fn()}
      goTrainer={vi.fn()}
    />,
  );
  const requestOp = (r: { body: Request }) =>
    (
      r.body as unknown as { request?: { op?: string; kind?: string } }
    ).request?.op;
  const trainingIndex = requests.findIndex(
    (r) => requestOp(r) === "training_state",
  );
  const quizzesIndex = requests.findIndex(
    (r) =>
      requestOp(r) === "items" &&
      (
        r.body as unknown as { request?: { kind?: string } }
      ).request?.kind === "range_quiz",
  );
  const leaksIndex = requests.findIndex((r) => requestOp(r) === "leaks");
  await respond(trainingIndex, {
    total: 20,
    due: 3,
    completed: 12,
    active: null,
    recent: [
      {
        answer: { mode: "action", action: "call", note: "" },
        feedback: { chosen: "call", in_strategy: null, max_gap: 4.2 },
        expected: null,
        observed: "bet 5bb",
        source_note: "",
        source: "self",
        pack: null,
        reference: {
          profile: "p",
          hand_id: "SYN1",
          seq: 0,
          version: "hero-decisions/2",
        },
        replay_hand: 7,
        rating: "advance",
        created: 1793500000,
      },
    ],
  });
  await respond(quizzesIndex, [
    {
      id: "range_quiz-1",
      data: {
        name: "p1:n1",
        pack: "p1",
        node: "n1",
        count: 10,
        avg: 88.5,
        created: 1793500000,
      },
    },
  ]);
  await respond(leaksIndex, {
    rows: [
      row({ id: "a", priority: 9, actual: 70 }),
      row({ id: "b", priority: 3, actual: 52, enough: false }),
    ],
    coverage: { total: 1, indexed: 1, complete: true, version: "hero-index/3" },
  });
  await waitFor(() =>
    expect(screen.getByText("Range 操練記錄")).toBeTruthy(),
  );
  expect(screen.getByText("20")).toBeTruthy();
  expect(screen.getByText("3")).toBeTruthy();
  expect(screen.getByText("12")).toBeTruthy();
  expect(screen.getAllByText("已達標").length).toBe(1);
  expect(screen.getAllByText("樣本不足").length).toBe(2);
  expect(screen.getAllByText("超出目標").length).toBe(2);
  expect(screen.getByText("SYN1")).toBeTruthy();
  expect(screen.getByText("88.5")).toBeTruthy();
  expect(screen.getByText("70.0%")).toBeTruthy();
  expect(screen.getAllByText("50–60%").length).toBe(2);
});
