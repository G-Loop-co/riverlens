// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { AiCoach } from "./components/AiCoach";
import "./i18n";
const native = vi.hoisted(() => ({ emit: (_: any) => {}, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: native.invoke,
  isTauri: () => true,
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_: string, callback: any) => {
    native.emit = callback;
    return () => {};
  }),
}));
vi.mock("./api", async () => ({
  ...(await vi.importActual("./api")),
  desktop: true,
  api: vi.fn(async (req: any) =>
    req.op === "profiles"
      ? []
      : {
          data:
            req.name === "get_learning_progress"
              ? { drafts: [], attempts: [] }
              : [],
        },
  ),
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
async function setup() {
  native.invoke.mockResolvedValue({});
  render(<AiCoach filter={{}} openHand={() => {}} />);
  fireEvent.change(screen.getByLabelText("模型供應商"), {
    target: { value: "deepseek" },
  });
  fireEvent.change(screen.getByLabelText("模型 ID"), {
    target: { value: "deepseek-flash" },
  });
  fireEvent.change(screen.getByLabelText("想改善哪個局面？"), {
    target: { value: "Review my hands" },
  });
  fireEvent.click(
    screen.getByLabelText("允許將問題與所需匿名化資料傳送至所選模型供應商。"),
  );
  await waitFor(() =>
    expect(
      (screen.getByRole("button", { name: "開始分析" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
}
function chat() {
  return native.invoke.mock.calls
    .filter(([name]) => name === "ai_chat")
    .at(-1)![1].chat;
}
it("keeps followups in the same session and resets history on model changes", async () => {
  await setup();
  fireEvent.click(screen.getByRole("button", { name: "開始分析" }));
  const first = chat();
  expect((screen.getByLabelText("模型 ID") as HTMLSelectElement).disabled).toBe(
    true,
  );
  act(() => {
    native.emit({
      payload: { id: first.id, type: "text", text: "Visible answer" },
    });
    native.emit({ payload: { id: first.id, type: "done" } });
  });
  fireEvent.click(screen.getByRole("button", { name: "開始分析" }));
  const second = chat();
  expect(second.session_id).toBe(first.session_id);
  expect(second.history).toEqual([
    { role: "user", text: "Review my hands" },
    { role: "assistant", text: "Visible answer" },
  ]);
  act(() => native.emit({ payload: { id: second.id, type: "done" } }));
  fireEvent.change(screen.getByLabelText("模型 ID"), {
    target: { value: "deepseek-v4-pro" },
  });
  fireEvent.click(screen.getByRole("button", { name: "開始分析" }));
  expect(chat().session_id).not.toBe(first.session_id);
  expect(chat().history).toEqual([]);
});
it("does not commit failed turns into the next request history", async () => {
  await setup();
  fireEvent.click(screen.getByRole("button", { name: "開始分析" }));
  act(() =>
    native.emit({
      payload: {
        id: chat().id,
        type: "error",
        message: "provider disconnected",
      },
    }),
  );
  fireEvent.click(screen.getByRole("button", { name: "開始分析" }));
  expect(chat().history).toEqual([]);
});
