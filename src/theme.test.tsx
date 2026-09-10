// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "./i18n";
import { ThemeSelect } from "./components/ThemeSelect";
import { applyTheme, readTheme, THEME_KEY } from "./theme";

beforeEach(() => {
  const values = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  delete document.documentElement.dataset.theme;
});

it("applies each choice immediately and restores it after remount/startup", () => {
  const view = render(<ThemeSelect />);
  for (const [label, id] of [
    ["午夜藍", "midnight"],
    ["暖白", "paper"],
    ["森林綠", "forest"],
  ]) {
    fireEvent.click(screen.getByRole("radio", { name: new RegExp(label) }));
    expect(document.documentElement.dataset.theme).toBe(id);
    expect(localStorage.getItem(THEME_KEY)).toBe(id);
  }
  fireEvent.click(screen.getByRole("radio", { name: /暖白/ }));
  view.unmount();
  delete document.documentElement.dataset.theme;
  applyTheme(readTheme());
  render(<ThemeSelect />);
  expect(document.documentElement.dataset.theme).toBe("paper");
  expect(
    (screen.getByRole("radio", { name: /暖白/ }) as HTMLInputElement).checked,
  ).toBe(true);
});

it("falls back safely for unknown preferences and permits changes when storage is blocked", () => {
  localStorage.setItem(THEME_KEY, "obsolete-theme");
  expect(readTheme()).toBe("forest");
  vi.stubGlobal("localStorage", {
    getItem() {
      throw new Error("blocked");
    },
    setItem() {
      throw new Error("blocked");
    },
  });
  render(<ThemeSelect />);
  fireEvent.click(screen.getByRole("radio", { name: /午夜藍/ }));
  expect(document.documentElement.dataset.theme).toBe("midnight");
  expect(
    (screen.getByRole("radio", { name: /午夜藍/ }) as HTMLInputElement).checked,
  ).toBe(true);
  cleanup();
  render(<ThemeSelect />);
  expect(
    (screen.getByRole("radio", { name: /午夜藍/ }) as HTMLInputElement).checked,
  ).toBe(true);
});
