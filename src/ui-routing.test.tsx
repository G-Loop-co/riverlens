// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HashRouter } from "react-router";
import "axe-core";
import App from "./App";
import i18n, { changeLanguage } from "./i18n";
import { metricIds } from "./benchmarks";
import { RouteErrorBoundary } from "./components/RouteUI";

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
  void changeLanguage("zh-TW");
});

function renderApp() {
  return render(
    <HashRouter>
      <App />
    </HashRouter>,
  );
}

describe("workspace routing", () => {
  it("has no serious or critical accessibility violations in the seven workspaces", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise(() => {})),
    );
    const routes = [
      ["#/today", "牌局總覽"],
      ["#/hands/list", "牌局覆盤"],
      ["#/learn/spots", "學習工作台"],
      ["#/research/stats", "MDA 群體分析"],
      ["#/coach/situation", "AI 教練"],
      ["#/data/import", "資料中心"],
      ["#/settings/preferences", "設定"],
    ] as const;
    for (const [hash, title] of routes) {
      cleanup();
      window.history.replaceState(null, "", hash);
      renderApp();
      await waitFor(() =>
        expect(screen.getByRole("heading", { level: 1 }).textContent).toContain(
          title,
        ),
      );
      const axeInstance = (globalThis as { axe?: typeof import("axe-core") })
        .axe;
      if (!axeInstance) throw new Error("axe-core did not initialize");
      const results = await axeInstance.run(document.body, {
        resultTypes: ["violations"],
      });
      expect(
        results.violations
          .filter((violation) =>
            ["serious", "critical"].includes(violation.impact ?? ""),
          )
          .map((violation) => violation.id),
      ).toEqual([]);
    }
  }, 18000);

  it("redirects the root to Today and exposes six primary workspaces", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise(() => {})),
    );
    renderApp();
    await waitFor(() => expect(window.location.hash).toContain("#/today"));
    for (const name of [
      "牌局總覽",
      "手牌複盤",
      "學習訓練",
      "牌池研究",
      "AI 教練",
      "設定",
    ]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }
    expect(screen.getByRole("button", { name: "匯入與資料" })).toBeTruthy();
  });

  it("returns a benchmark drill to the original cohort, metric and filters", async () => {
    window.history.replaceState(null, "", "#/today");
    const rows = ["Cash", "Rush"].map((game) => ({
      currency: "USD",
      game,
      stakes: "0.01/0.02",
      position: "BB",
      hands: 20,
      metrics: Object.fromEntries(metricIds.map((id) => [id, [1, 10]])),
    }));
    vi.stubGlobal(
      "fetch",
      vi.fn((_url, init: RequestInit) => {
        if (JSON.parse(String(init.body)).op !== "review_dashboard")
          return new Promise(() => {});
        return Promise.resolve({
          ok: true,
          json: async () => ({
            result: { positions: rows, latest: null, has_hands: true },
          }),
        });
      }),
    );
    const saved = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => saved.get(key) ?? null,
      setItem: (key: string, value: string) => saved.set(key, value),
    });
    renderApp();
    const cohort = JSON.stringify(["USD", "Rush", "0.01/0.02"]);
    const select = await screen.findByRole("combobox", {
      name: new RegExp(i18n.t("review.cohort")),
    });
    fireEvent.change(select, { target: { value: cohort } });
    const metricButton = () =>
      screen
        .getByText(i18n.t("bench.three_bet"), { selector: "strong" })
        .closest("button")!;
    fireEvent.click(metricButton());
    fireEvent.click(
      screen.getByRole("button", { name: i18n.t("review.missed") }),
    );
    expect(window.location.hash).toContain("stat_mode=benchmark_missed");
    fireEvent.click(
      await screen.findByRole("button", {
        name: i18n.t("← 返回報表及原本篩選"),
      }),
    );
    const restored = await screen.findByRole<HTMLSelectElement>("combobox", {
      name: new RegExp(i18n.t("review.cohort")),
    });
    expect(restored.value).toBe(cohort);
    expect(metricButton().getAttribute("aria-expanded")).toBe("true");
    expect(window.location.hash).not.toContain("stat_mode");
  }, 18000);

  it("restores a hand-review tab from a deep link", () => {
    window.history.replaceState(null, "", "#/hands/matrix?currency=USD");
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise(() => {})),
    );
    renderApp();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toContain(
      "起手牌矩陣",
    );
    expect(screen.getByRole("tab", { selected: true }).textContent).toContain(
      "起手牌矩陣",
    );
  });

  it("localizes every sidebar workspace in English", async () => {
    await changeLanguage("en");
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise(() => {})),
    );
    renderApp();
    await waitFor(() => expect(window.location.hash).toContain("#/today"));
    for (const name of [
      "Overview",
      "Hand review",
      "Study & practice",
      "Pool research",
      "AI coach",
      "Import & data",
      "Settings",
    ]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }
  });

  it("keeps the rest of the shell usable when a workspace throws", () => {
    function Broken(): never {
      throw new Error("workspace failed");
    }
    render(
      <RouteErrorBoundary>
        <Broken />
      </RouteErrorBoundary>,
    );
    expect(screen.getByRole("alert").textContent).toContain("workspace failed");
    expect(screen.getByRole("button", { name: "重試" })).toBeTruthy();
  });
});
