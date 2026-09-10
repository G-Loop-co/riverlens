import type { Filter } from "./types";

export function readPreference(key: string, fallback = ""): string {
  try {
    return localStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}

export function writePreference(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Session state remains usable when the WebView blocks persistent storage.
  }
}

export function readFilter(): Filter {
  try {
    const value: unknown = JSON.parse(readPreference("riverlens-filter", "{}"));
    return normalizeFilter(value);
  } catch {
    return {};
  }
}

// SQLite/Rust saved filters include null Option fields; UI state uses omissions.
export function normalizeFilter(value: unknown): Filter {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const numbers = new Set([
    "player_count",
    "stack_min",
    "stack_max",
    "effective_min",
    "effective_max",
    "flop_players",
    "bet_min",
    "bet_max",
  ]);
  const booleans = new Set(["paired", "showdown", "reviewed"]);
  const strings = new Set([
    "profile",
    "source_date",
    "date_from",
    "date_to",
    "timezone",
    "position",
    "game",
    "stakes",
    "currency",
    "pot_type",
    "hand_class",
    "texture",
    "high_card",
    "stat",
    "stat_mode",
    "cue",
    "street",
    "action",
    "tag",
    "search",
    "status",
    "ev_status",
    "session",
    "result",
  ]);
  return Object.fromEntries(
    Object.entries(value).filter(([key, entry]) =>
      numbers.has(key)
        ? typeof entry === "number" && Number.isFinite(entry) && entry >= 0
        : booleans.has(key)
          ? typeof entry === "boolean"
          : strings.has(key) && typeof entry === "string" && entry.length > 0,
    ),
  );
}
