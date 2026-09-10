// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { AiCoach } from "./components/AiCoach";
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
