import { describe, expect, it } from "vitest";
import { fromUnits, replay, units } from "./replay";
import type { Action, Hand } from "./types";
function action(
  kind: string,
  actor: number | null,
  amount: string,
  cards: string[] = [],
  runout = 0,
): Action {
  return {
    seq: 0,
    kind,
    actor,
    amount,
    cards,
    runout,
    street: cards.length ? "flop" : "preflop",
    to: "0",
    pot_after: "0",
    all_in: false,
  };
}
function hand(actions: Action[]): Hand {
  return {
    players: [
      { seat: 1, hero: true, stack: "2.00" },
      { seat: 2, hero: false, stack: "2.00" },
    ],
    actions,
  } as Hand;
}
describe("exact chronological replayer", () => {
  it("roundtrips signed micro units without floats", () => {
    for (const n of [0n, 1n, -1n, 1_234_567n, 999_999_999_999_999n])
      expect(units(fromUnits(n))).toBe(n);
  });
  it("does not expose future opponent cards or boards", () => {
    const h = hand([
      action("board", null, "0", ["As", "2h", "3h"]),
      action("show", 2, "0", ["Ks", "Kd"]),
    ]);
    expect(replay(h, 0).boards).toEqual([[]]);
    expect(replay(h, 1).shown.has(2)).toBe(false);
    expect(replay(h, 2).shown.has(2)).toBe(true);
  });
  it("tracks returns and collection exactly once", () => {
    const h = hand([
      action("raise", 1, ".06"),
      action("big_blind", 2, ".02"),
      action("return", 1, ".04"),
      action("collect", 1, ".04"),
    ]);
    const final = replay(h, 4);
    expect(final.stacks[1]).toBe(units("2.02"));
    expect(final.stacks[2]).toBe(units("1.98"));
    expect(final.pot).toBe(0n);
    expect(replay(h, 1).pot).toBe(units(".06"));
  });
  it("cashout settlement never changes the table pot", () => {
    const h = hand([
      action("bet", 1, "1"),
      action("cashout_receive", 1, ".8"),
      action("cashout_risk", 1, ".2"),
    ]);
    expect(replay(h, 3).pot).toBe(units("1"));
    expect(replay(h, 3).stacks[1]).toBe(units("1.6"));
  });
  it("retains separate runouts, resets street contributions", () => {
    const h = hand([
      action("bet", 1, ".1"),
      action("board", null, "0", ["As", "2h", "3h"]),
      action("board", null, "0", ["Ks", "2d", "3d"], 1),
    ]);
    const r = replay(h, 3);
    expect(r.boards).toHaveLength(2);
    expect(r.contributions).toEqual({});
    expect(r.pot).toBe(units(".1"));
  });
});
