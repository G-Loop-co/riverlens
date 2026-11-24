import type { Filter } from "./types";

export const ranges = {
  vpip: [21, 27],
  pfr: [17, 23],
  three_bet: [6, 10],
  fold_three_bet: [45, 65],
  cbet: [50, 70],
  fold_cbet: [40, 55],
} as const;
export type Metric = keyof typeof ranges;
export const metricIds = Object.keys(ranges) as Metric[];
export type Counts = [hits: number, opportunities: number];
export interface PositionSummary {
  currency: string;
  game: string;
  stakes: string;
  position: string;
  hands: number;
  metrics: Record<Metric, Counts>;
}
export interface ReviewSummary {
  positions: PositionSummary[];
  latest: {
    session: string;
    currency: string;
    time: number;
    hands: number;
    net: string;
    net_bb: number;
  } | null;
  has_hands: boolean;
  default_currency?: string | null;
}
export interface Cohort {
  key: string;
  currency: string;
  game: string;
  stakes: string;
  hands: number;
  positions: PositionSummary[];
  metrics: Record<Metric, Counts>;
}
export function cohorts(rows: PositionSummary[]): Cohort[] {
  const groups = new Map<string, Cohort>();
  for (const row of rows) {
    const key = JSON.stringify([row.currency, row.game, row.stakes]);
    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        currency: row.currency,
        game: row.game,
        stakes: row.stakes,
        hands: 0,
        positions: [],
        metrics: Object.fromEntries(
          metricIds.map((id) => [id, [0, 0]]),
        ) as Record<Metric, Counts>,
      };
      groups.set(key, group);
    }
    group.positions.push(row);
    group.hands += row.hands;
    for (const id of metricIds) {
      group.metrics[id][0] += row.metrics[id][0];
      group.metrics[id][1] += row.metrics[id][1];
    }
  }
  return [...groups.values()].sort(
    (a, b) => b.hands - a.hands || a.key.localeCompare(b.key),
  );
}

export function allowsOverallBand(filter: Filter) {
  const scope = new Set([
    "currency",
    "game",
    "stakes",
    "profile",
    "player_count",
    "date_from",
    "date_to",
    "timezone",
    "source_date",
    "session",
  ]);
  return (
    Object.entries(filter).every(
      ([key, value]) => value == null || scope.has(key),
    ) &&
    (!filter.player_count || filter.player_count === 6)
  );
}
export function assess(id: Metric, [hits, n]: Counts) {
  if (!n) return { value: null, gap: 0, priority: false };
  const p = hits / n,
    value = (100 * hits) / n;
  const [low, high] = ranges[id];
  const gap =
    100 * hits < low * n
      ? value - low
      : 100 * hits > high * n
        ? value - high
        : 0;
  const z = 1.959963984540054,
    z2 = z * z;
  const center = p + z2 / (2 * n),
    margin = z * Math.sqrt((p * (1 - p) + z2 / (4 * n)) / n),
    den = 1 + z2 / n;
  return {
    value,
    gap,
    priority:
      n >= 100 &&
      (((center - margin) / den) * 100 > high ||
        ((center + margin) / den) * 100 < low),
  };
}

export function priorities(groups: Cohort[], filter: Filter) {
  if (!allowsOverallBand(filter)) return [];
  return groups
    .flatMap((group) =>
      metricIds.map((id) => ({ group, id, ...assess(id, group.metrics[id]) })),
    )
    .filter((row) => row.priority)
    .sort(
      (a, b) =>
        Math.abs(b.gap) - Math.abs(a.gap) ||
        b.group.metrics[b.id][1] - a.group.metrics[a.id][1],
    )
    .slice(0, 3);
}
