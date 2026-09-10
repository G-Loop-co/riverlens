import type { Hand } from "./types";

export function units(s: string): bigint {
  const negative = s.startsWith("-");
  const [whole, fraction = ""] = (negative ? s.slice(1) : s).split(".");
  return (
    (BigInt(whole) * 1_000_000n + BigInt(fraction.padEnd(6, "0"))) *
    (negative ? -1n : 1n)
  );
}
export function fromUnits(n: bigint): string {
  const negative = n < 0n;
  const a = negative ? -n : n;
  return `${negative ? "-" : ""}${a / 1_000_000n}.${(a % 1_000_000n).toString().padStart(6, "0")}`;
}
export function replay(hand: Hand, count: number) {
  const stacks: Record<number, bigint> = {};
  let contributions: Record<number, bigint> = {};
  const folded = new Set<number>();
  const shown = new Set<number>();
  let pot = 0n;
  const boards: string[][] = [[]];
  let street = "preflop";
  let runout = 0;
  hand.players.forEach((p) => {
    stacks[p.seat] = units(p.stack);
    contributions[p.seat] = 0n;
    if (p.hero) shown.add(p.seat);
  });
  for (const action of hand.actions.slice(0, count)) {
    if (action.kind === "board") {
      street = action.street;
      runout = action.runout;
      boards[runout] = action.cards;
      contributions = {};
      continue;
    }
    if (action.actor === null) continue;
    const seat = action.actor,
      amount = units(action.amount);
    if (
      [
        "small_blind",
        "big_blind",
        "dead_blind",
        "ante",
        "straddle",
        "call",
        "bet",
        "raise",
      ].includes(action.kind)
    ) {
      stacks[seat] -= amount;
      pot += amount;
      if (!["dead_blind", "ante"].includes(action.kind))
        contributions[seat] = (contributions[seat] ?? 0n) + amount;
    } else if (action.kind === "return") {
      stacks[seat] += amount;
      pot -= amount;
      contributions[seat] = (contributions[seat] ?? 0n) - amount;
    } else if (action.kind === "collect") {
      stacks[seat] += amount;
      pot -= amount;
    } else if (action.kind === "cashout_receive") stacks[seat] += amount;
    else if (action.kind === "cashout_risk") stacks[seat] -= amount;
    else if (action.kind === "fold") folded.add(seat);
    else if (action.kind === "show") shown.add(seat);
  }
  return { stacks, contributions, folded, shown, pot, boards, street, runout };
}
