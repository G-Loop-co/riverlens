// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { Trainer } from "./components/StudyTrainer";
import { StudySelect } from "./components/StudyWorkspace";
import { SpotEditor } from "./components/StudySpots";
import i18n from "./i18n";
import type { Request } from "./types";

afterEach(async () => {
  cleanup();
  vi.unstubAllGlobals();
  await i18n.changeLanguage("zh-TW");
});
it("preserves user labels and canonical spot values across languages", async () => {
  await i18n.changeLanguage("en");
  const changed = vi.fn();
  render(
    <>
      <StudySelect
        label="study.savedSpots"
        value="custom"
        all={false}
        options={[["custom", "牌局總覽"]]}
        change={changed}
      />
      <SpotEditor
        spot={{ street: "flop", paired: false, line: [] }}
        setSpot={changed}
      />
    </>,
  );
  expect(screen.getByRole("option", { name: "牌局總覽" })).toBeTruthy();
  expect(
    (screen.getByRole("combobox", { name: "Street" }) as HTMLSelectElement)
      .value,
  ).toBe("flop");
  fireEvent.change(screen.getByRole("combobox", { name: "Paired flop" }), {
    target: { value: "true" },
  });
  expect(changed).toHaveBeenLastCalledWith({
    street: "flop",
    paired: true,
    line: [],
  });
});

it("requires a complete frequency answer, prevents duplicate submissions and reveals replay only afterward", async () => {
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
  const respond = async (index: number, result: unknown) =>
    act(async () =>
      requests[index].resolve({
        ok: true,
        json: async () => ({ result }),
      } as Response),
    );
  const open = vi.fn();
  render(
    <Trainer revision={0} refresh={vi.fn()} open={open} findSpots={vi.fn()} />,
  );
  await respond(0, {
    total: 1,
    due: 1,
    completed: 0,
    active: null,
    recent: [],
  });
  fireEvent.click(screen.getByRole("button", { name: "開始練習" }));
  const question = {
    id: "session",
    card_id: "card",
    index: 0,
    total: 1,
    complete: false,
    source: "gto",
    feedback: null,
    stale: false,
    question: {
      street: "preflop",
      position: "UTG",
      cards: ["As", "Ah"],
      board: [],
      line: [],
      preflop_path: "",
      options: [
        { code: "F", label: "Fold" },
        { code: "R2.5", label: "Raise 2.5" },
      ],
    },
  };
  await respond(1, question);
  expect(screen.queryByRole("button", { name: "開啟完整回放" })).toBeNull();
  const submit = screen.getByRole("button", {
    name: "提交並揭示答案",
  }) as HTMLButtonElement;
  expect(submit.disabled).toBe(true);
  fireEvent.change(screen.getByRole("spinbutton", { name: "Fold %" }), {
    target: { value: "40" },
  });
  fireEvent.change(screen.getByRole("spinbutton", { name: "Raise 2.5 %" }), {
    target: { value: "60" },
  });
  expect(submit.disabled).toBe(false);
  fireEvent.click(submit);
  fireEvent.click(submit);
  expect(requests.length).toBe(3);
  expect(requests[2].body).toMatchObject({
    op: "study",
    request: {
      op: "training_answer",
      answer: { mode: "frequency", frequencies: { F: 40, "R2.5": 60 } },
    },
  });
  await respond(2, {
    ...question,
    feedback: {
      feedback: { max_gap: 10, differences: { F: -10, "R2.5": 10 } },
      expected: { F: 0.5, "R2.5": 0.5 },
      observed: "raise 2.5bb",
      answer: { note: "" },
      source_note: "",
      reference: { seq: 7 },
      replay_hand: 42,
    },
  });
  fireEvent.click(screen.getByRole("button", { name: "開啟完整回放" }));
  expect(open).toHaveBeenCalledWith(42, 7);
});
