import { readPreference, writePreference } from "./preferences";

export const THEME_KEY = "riverlens-theme";
export const themes = [
  { id: "forest", label: "森林綠", description: "柔和綠調，經典深色工作台。" },
  { id: "midnight", label: "午夜藍", description: "冷調藍灰，清晰專注。" },
  { id: "paper", label: "暖白", description: "溫暖明亮，適合日間閱讀。" },
] as const;
export type Theme = (typeof themes)[number]["id"];

export function supportedTheme(value: string): Theme {
  return themes.find((theme) => theme.id === value)?.id ?? "paper";
}

export function readTheme(): Theme {
  return supportedTheme(readPreference(THEME_KEY));
}

export function applyTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute(
      "content",
      { forest: "#151b18", midnight: "#131a26", paper: "#f4f1ea" }[theme],
    );
}

export function selectTheme(theme: Theme) {
  applyTheme(theme);
  writePreference(THEME_KEY, theme);
}
