// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HashRouter } from "react-router";
import "axe-core";
import App from "./App";
import { changeLanguage } from "./i18n";
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
  });

  it("redirects the root to Today and exposes six primary workspaces", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise(() => {})),
    );
    renderApp();
    await waitFor(() => expect(window.location.hash).toContain("#/today"));
    for (const name of [
      "牌局總覽",
      "牌局覆盤",
      "學習工作台",
      "MDA 群體分析",
      "AI 教練",
      "設定",
    ]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }
    expect(screen.getByRole("button", { name: "資料中心" })).toBeTruthy();
  });

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
      "Study workspace",
      "MDA population analysis",
      "AI coach",
      "Data center",
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
