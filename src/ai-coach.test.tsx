// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { AiCoach, EvidenceCard } from "./components/AiCoach";
import * as backend from "./api";
import "./i18n";
vi.mock("./api", async () => ({
  ...(await vi.importActual("./api")),
  api: vi.fn(),
  desktop: false,
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
function engine() {
  let drafts: any[] = [];
  vi.mocked(backend.api).mockImplementation(async (req: any): Promise<any> => {
    if (req.op === "profiles") return [];
    if (req.op === "agent_review") {
      drafts[0].status = req.accept ? "accepted" : "draft";
      return { saved: true };
    }
    if (req.op === "agent_tool") {
      if (req.name === "get_learning_progress")
        return { data: { drafts: [...drafts], attempts: [] } };
      if (req.name === "list_strategy_packs") return { data: [] };
      if (req.name === "query_stats")
        return {
          evidence_id: "e-1",
          version: "v1",
          tool: req.name,
          args: req.arguments,
          data: { hands: 3, stats: [] },
        };
      if (req.name === "create_report_draft") {
        drafts.push({
          id: req.arguments.draft.id,
          title: req.arguments.draft.title,
          body: req.arguments.draft,
          status: "draft",
          revision: 1,
        });
        return { data: { saved: true } };
      }
      if (req.name === "find_spots")
        return {
          evidence_id: "e-2",
          version: "v1",
          tool: req.name,
          args: req.arguments,
          data: {
            rows: [{ id: 42, position: "BTN", hand_class: "AA" }],
            next_cursor: null,
          },
        };
    }
    throw new Error("Unexpected test call " + JSON.stringify(req));
  });
}
it("creates a local draft and requires explicit user acceptance", async () => {
  engine();
  render(<AiCoach filter={{ position: "BB" }} openHand={() => {}} />);
  await waitFor(() => expect(backend.api).toHaveBeenCalled());
  expect(
    (screen.getByRole("button", { name: "開始分析" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "建立本機統計草稿" }));
  await waitFor(() =>
    expect(backend.api).toHaveBeenCalledWith(
      expect.objectContaining({ name: "create_report_draft" }),
    ),
  );
  fireEvent.click(screen.getByRole("button", { name: "學習資料" }));
  await screen.findByText("目前篩選統計");
  expect(backend.api).not.toHaveBeenCalledWith(
    expect.objectContaining({ op: "agent_review" }),
  );
  fireEvent.click(screen.getByRole("button", { name: "編輯與確認" }));
  fireEvent.click(screen.getByRole("button", { name: "接受並儲存" }));
  await waitFor(() =>
    expect(backend.api).toHaveBeenCalledWith(
      expect.objectContaining({
        op: "agent_review",
        accept: true,
        revision: 1,
      }),
    ),
  );
});
it("passes ordered action filters and opens source hands", async () => {
  engine();
  const open = vi.fn();
  render(<AiCoach filter={{ position: "BB" }} openHand={open} />);
  fireEvent.click(screen.getByRole("button", { name: "局面探索" }));
  fireEvent.click(screen.getByRole("button", { name: "新增行動" }));
  fireEvent.click(screen.getByRole("button", { name: "搜尋局面" }));
  fireEvent.click(await screen.findByRole("button", { name: "BTN AA #42" }));
  expect(open).toHaveBeenCalledWith(42);
  expect(backend.api).toHaveBeenCalledWith(
    expect.objectContaining({
      name: "find_spots",
      arguments: expect.objectContaining({
        filter: { position: "BB" },
        path: [{ street: "preflop", position: "BTN", kind: "raise" }],
      }),
    }),
  );
});

it("selects new providers in coach and connections and clears incompatible model", async () => {
  engine();
  render(<AiCoach filter={{}} openHand={() => {}} />);
  await waitFor(() => expect(backend.api).toHaveBeenCalled());
  const provider = screen.getByLabelText("模型供應商") as HTMLSelectElement;
  const model = screen.getByLabelText("模型 ID") as HTMLSelectElement;
  expect(model.tagName).toBe("SELECT");
  expect(screen.queryByRole("textbox", { name: "模型 ID" })).toBeNull();
  fireEvent.change(model, { target: { value: "gpt-4.1" } });
  for (const name of ["deepseek", "opencode-go"]) {
    fireEvent.change(provider, { target: { value: name } });
    expect(provider.value).toBe(name);
    expect(model.value).toBe("");
    expect(
      Array.from(model.options).some((option) => option.value === "gpt-4.1"),
    ).toBe(false);
    const choice = model.options[1].value;
    fireEvent.change(model, { target: { value: choice } });
    expect(model.value).toBe(choice);
  }
  fireEvent.click(screen.getByRole("button", { name: "AI 連接" }));
  expect(screen.getByRole("option", { name: "deepseek" })).toBeTruthy();
  expect(screen.getByRole("option", { name: "opencode-go" })).toBeTruthy();
  fireEvent.change(screen.getByLabelText("模型供應商"), {
    target: { value: "gemini" },
  });
  fireEvent.click(screen.getByRole("button", { name: "AI 教練" }));
  const updated = screen.getByLabelText("模型 ID") as HTMLSelectElement;
  expect(updated.value).toBe("");
  expect(Array.from(updated.options).map((option) => option.value)).toEqual([
    "",
    "gemini-2.5-flash",
    "gemini-2.5-pro",
  ]);
});

for (const [tool, data] of [
  [
    "query_stats",
    { hands: { total: 10 }, stats: { vpip: 2 }, rows: { position: "BTN" } },
  ],
  ["get_hand", { id: "external-hand", stats: { vpip: true }, status: "valid" }],
  ["get_metric_definitions", [{ id: "vpip", label: "VPIP" }]],
  ["list_strategy_packs", null],
  ["search_learning", { rows: [{ id: "report-id", text: "source report" }] }],
] as const) {
  it(`renders saved ${tool} evidence without crashing or inventing hand links`, () => {
    render(
      <EvidenceCard
        e={{
          evidence_id: "e-test",
          version: "v1",
          tool,
          args: tool === "get_hand" ? { id: 42 } : {},
          data: data as any,
        }}
        openHand={() => {}}
      />,
    );
    expect(screen.getByText("完整證據內容")).toBeTruthy();
    if (tool === "get_hand")
      expect(screen.getByRole("button", { name: "打開回放 #42" })).toBeTruthy();
    if (tool === "search_learning")
      expect(screen.queryByRole("button")).toBeNull();
  });
}
it("opens saved evidence using the original envelope", async () => {
  engine();
  const original = vi.mocked(backend.api).getMockImplementation()!;
  vi.mocked(backend.api).mockImplementation(async (req: any) =>
    req.op === "agent_evidence"
      ? {
          evidence_id: "e-1",
          version: "v1",
          tool: "get_metric_definitions",
          args: {},
          data: [{ id: "vpip" }],
        }
      : original(req),
  );
  render(<AiCoach filter={{}} openHand={() => {}} />);
  fireEvent.click(screen.getByRole("button", { name: "建立本機統計草稿" }));
  await waitFor(() =>
    expect(backend.api).toHaveBeenCalledWith(
      expect.objectContaining({ name: "create_report_draft" }),
    ),
  );
  fireEvent.click(screen.getByRole("button", { name: "學習資料" }));
  fireEvent.click(
    await screen.findByRole("button", { name: "查看證據 1", hidden: true }),
  );
  expect(
    await screen.findByText("資料證據 · get_metric_definitions"),
  ).toBeTruthy();
  expect(screen.getByText('"vpip"', { exact: false })).toBeTruthy();
});
