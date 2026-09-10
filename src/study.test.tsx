import { LeakFinder } from "./components/StudyLeaks";
import { StrategyDecisions } from "./components/StudyStrategyDecisions";
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


it("shows observation-only frequencies without inventing target or priority and preserves names", async () => {
  await i18n.changeLanguage("en");
  const row = {id:"observation", name:"牌局總覽", source:"observation", spot:{line:[]}, action:"raise", actual:60, low:null, high:null, gap:null, interval:[50,70], opportunities:120,hits:72,enough:true,priority:null,note:""};
  vi.stubGlobal("fetch",vi.fn(async (_url,options) => ({ok:true,json:async()=>({result:JSON.parse(options.body).request.op==="leaks"?{rows:[row]}:[]})})));
  const explore=vi.fn();
  render(<LeakFinder filter={{}} spot={{line:[]}} pack="" revision={0} refresh={vi.fn()} explore={explore} strategy={vi.fn()}/>);
  await screen.findByText("No target");
  expect(screen.getByText("Observation only; no correctness judgement")).toBeTruthy();
  expect(screen.getByText("牌局總覽")).toBeTruthy();
  expect(screen.queryByText("Review priority")).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"View decisions"}));
  expect(explore).toHaveBeenCalledWith({line:[]});
});

it("keeps strategy trace matching and reference cohorts separate and pages by cursor",async()=>{
  const requests:Request[]=[];
  vi.stubGlobal("fetch",vi.fn(async (_url,options)=>{
    const body=JSON.parse(options.body); requests.push(body);
    return {ok:true,json:async()=>({result:{opportunities:120,rows:[],next_cursor:55}})};
  }));
  render(<StrategyDecisions pack="p" node="UTG:" hand="AA" filter={{}} revision={0} open={vi.fn()} notify={vi.fn()} refresh={vi.fn()}/>);
  await screen.findByRole("button",{name:"載入更多"});
  expect(requests[0]).toMatchObject({request:{query:{strategy:{pack:"p",node:"UTG:",matched_only:true}}}});
  fireEvent.click(screen.getByRole("checkbox",{name:"只看配置匹配手牌"}));
  await waitFor(()=>expect(requests.at(-1)).toMatchObject({request:{query:{strategy:{matched_only:false},before:null}}}));
  fireEvent.click(await screen.findByRole("button",{name:"載入更多"}));
  await waitFor(()=>expect(requests.at(-1)).toMatchObject({request:{query:{before:55}}}));
  fireEvent.click(screen.getByRole("checkbox",{name:"限所選牌組 AA"}));
  await waitFor(()=>expect(requests.at(-1)).toMatchObject({request:{query:{filter:{hand_class:"AA"},before:null}}}));
});
