// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { HandFormats, importRooms } from "./components/HandFormats";
import { currencySymbol } from "./format";

afterEach(cleanup);

it("makes unsupported native exports visible without offering them as native support", () => {
  render(<HandFormats />);
  fireEvent.click(screen.getByText("支援格式與匯出指南"));
  for (const label of [
    "WPT Global (converted)",
    "Club apps (converted)",
    "Chico (converted)",
  ]) {
    const link = screen.getByRole("link", { name: label });
    const row = link.closest("tr")!;
    expect(within(row).getByText("僅 PokerStars 轉換檔")).toBeTruthy();
    expect(link.getAttribute("href")).toMatch(/^https:\/\//);
  }
  expect(importRooms.find(({ room }) => room === "WPT Global")?.converted).toBe(
    true,
  );
  expect(importRooms.some(({ room }) => room === "Boss Media / IPN")).toBe(
    false,
  );
  expect(
    screen.getByRole("link", { name: "Boss Media / IPN" }).closest("tr")
      ?.textContent,
  ).toContain("尚未支援");
});

it("renders actual currency labels instead of labelling every result USD", () => {
  expect(currencySymbol("EUR")).toBe("€");
  expect(currencySymbol("GBP")).toBe("£");
  expect(currencySymbol("USDT")).toBe("₮");
  expect(currencySymbol("CAD")).toBe("CA$");
  expect(currencySymbol("CHIPS")).toBe("");
});

it("lists tournament support separately from cash room support", () => {
  render(<HandFormats />);
  fireEvent.click(screen.getByText("支援格式與匯出指南"));
  for (const label of [
    "GG / Natural8",
    "PokerStars",
    "Winning Poker Network",
    "Winamax",
    "888 / Pacific",
    "iPoker",
    "Full Tilt (archive)",
  ]) {
    const row = screen.getByRole("link", { name: label }).closest("tr")!;
    expect(within(row).getByText("Cash ＋ 錦標賽 · 樣本測試")).toBeTruthy();
  }
  expect(
    screen.getByRole("link", { name: "CoinPoker" }).closest("tr")?.textContent,
  ).toContain("錦標賽未支援");
});
