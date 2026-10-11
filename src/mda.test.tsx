// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { MdaWorkspace } from "./components/MdaWorkspace";
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
  vi.unstubAllGlobals();
});
it("keeps datasets separate, pins queries and requires dataset cloud consent", async () => {
  vi.mocked(backend.api).mockImplementation(async (req: any): Promise<any> => {
    if (req.op === "agent_learning") return { drafts: [], attempts: [] };
    if (req.op !== "mda") throw Error("Unexpected personal query");
    const r = req.request;
    if (r.op === "list")
      return {
        datasets: [
          {
            dataset: {
              id: "mda-test",
              name: "Test pool",
              timezone: "UTC",
              cloud_enabled: false,
              mcp_enabled: false,
            },
            version: "mda:test:1",
            hands: 1000,
            quarantined: 0,
            stakes: [{ bb_units: 250000, hands: 1000 }],
            metrics: [{ key: "fold_cbet" }],
          },
        ],
      };
    if (r.op === "jobs") return { jobs: [], running: false };
    if (r.op === "query")
      return { version: "mda:test:1", data: { rows: [], next_cursor: null } };
    throw Error("unexpected request");
  });
  vi.stubGlobal("fetch", async (_url: unknown, init: RequestInit) => ({
    ok: true,
    json: async () => ({
      result: await backend.api(JSON.parse(init.body as string)),
    }),
  }));
  render(<MdaWorkspace />);
  await screen.findByText("1,000 有效手牌", { exact: false });
  await waitFor(() =>
    expect(backend.api).toHaveBeenCalledWith(
      expect.objectContaining({
        op: "mda",
        request: expect.objectContaining({
          op: "query",
          dataset_id: "mda-test",
          arguments: expect.objectContaining({
            version: "mda:test:1",
            filter: { hu_postflop: true },
          }),
        }),
      }),
    ),
  );
  fireEvent.click(screen.getByRole("button", { name: "AI 剝削分析" }));
  expect(
    await screen.findByText("在「平台限制及資料分享」", { exact: false }),
  ).toBeTruthy();
  expect(screen.queryByRole("button", { name: "開始分析" })).toBeNull();
  expect(screen.getByRole("heading", { name: "學習資料" })).toBeTruthy();
  const share = screen.getByLabelText(
    "允許此資料集供雲端 AI Coach 分析",
  ) as HTMLInputElement;
  expect(share.checked).toBe(false);
});
it("anonymous evidence opens an opaque population key rather than personal hand IDs", () => {
  const open = vi.fn(),
    personal = vi.fn();
  render(
    <EvidenceCard
      e={{
        evidence_id: "e",
        version: "mda:test:1",
        tool: "find_mda_spots",
        args: {},
        data: {
          rows: [
            {
              hand_key: "000000000000000a",
              seq: 9,
              position: "BB",
              street: "river",
              board: "Ks 7d 2c 3h 9c",
            },
          ],
        },
      }}
      openHand={personal}
      openMdaHand={open}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "匿名回放" }));
  expect(open).toHaveBeenCalledWith("000000000000000a", 9);
  expect(personal).not.toHaveBeenCalled();
});

it("keeps saved MDA drafts and anonymous evidence available after cloud sharing is disabled", async () => {
  const open = vi.fn();
  const body = {
    id: "mda-report",
    title: "Saved MDA report",
    text: "適用情境\n數據與分母\n調整方向\n理由\n例外\n匿名手牌證據",
    items: [],
    evidence: ["e-mda"],
  };
  const draft = {
    id: body.id,
    title: body.title,
    body,
    status: "draft",
    revision: 1,
  };
  vi.mocked(backend.api).mockImplementation(async (req: any): Promise<any> => {
    if (req.op === "agent_learning") return { drafts: [draft], attempts: [] };
    if (req.op === "agent_review") return { saved: true };
    if (req.op === "agent_evidence")
      return {
        evidence_id: "e-mda",
        version: "mda:mda-test:1",
        tool: "get_mda_hand",
        args: { dataset_id: "mda-test", hand_key: "000000000000000a" },
        data: {},
      };
    throw new Error("Unexpected shared tool call");
  });
  render(
    <AiCoach
      filter={{}}
      openHand={() => {}}
      openMdaHand={open}
      mda={{
        dataset_id: "mda-test",
        version: "mda:mda-test:1",
        filter: { hu_postflop: true },
        cloud_enabled: false,
      }}
    />,
  );
  await screen.findByText("Saved MDA report");
  expect(screen.queryByRole("button", { name: "開始分析" })).toBeNull();
  expect(backend.api).toHaveBeenCalledWith({
    op: "agent_learning",
    dataset_id: "mda-test",
  });
  fireEvent.click(screen.getByRole("button", { name: "編輯與確認" }));
  fireEvent.click(screen.getByRole("button", { name: "接受並儲存" }));
  await waitFor(() =>
    expect(backend.api).toHaveBeenCalledWith(
      expect.objectContaining({ op: "agent_review", accept: true }),
    ),
  );
  fireEvent.click(screen.getByText("來源證據", { exact: false }));
  fireEvent.click(screen.getByRole("button", { name: "查看證據 mda" }));
  fireEvent.click(await screen.findByRole("button", { name: "匿名回放" }));
  expect(open).toHaveBeenCalledWith("000000000000000a", undefined);
  expect(backend.api).not.toHaveBeenCalledWith(
    expect.objectContaining({ op: "agent_tool" }),
  );
});
