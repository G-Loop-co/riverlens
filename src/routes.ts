import type { Filter } from "./types";

export const WORKSPACES = [
  "today",
  "hands",
  "learn",
  "research",
  "coach",
  "data",
  "settings",
] as const;
export type Workspace = (typeof WORKSPACES)[number];

export const DEFAULT_SECTIONS = {
  today: "dashboard",
  hands: "list",
  learn: "spots",
  research: "stats",
  coach: "situation",
  data: "import",
  settings: "preferences",
} as const;

const VALID_SECTIONS = {
  today: new Set(["dashboard"]),
  hands: new Set(["list", "matrix", "review"]),
  learn: new Set([
    "spots",
    "progress",
    "leaks",
    "strategy",
    "range",
    "trainer",
  ]),
  research: new Set([
    "stats",
    "wizard",
    "candidates",
    "evidence",
    "ai",
    "import",
  ]),
  coach: new Set(["situation", "chat", "library", "connections"]),
  data: new Set(["import", "sources"]),
  settings: new Set(["preferences"]),
} as const;

export type AppRoute = {
  workspace: Workspace;
  section: string;
  pathname: string;
  handId?: number;
};

const NUMBER_FILTER_KEYS = new Set([
  "player_count",
  "stack_min",
  "stack_max",
  "effective_min",
  "effective_max",
  "flop_players",
  "bet_min",
  "bet_max",
]);
const BOOLEAN_FILTER_KEYS = new Set(["paired", "showdown", "reviewed"]);
const STRING_FILTER_KEYS = new Set([
  "profile",
  "source_date",
  "date_from",
  "date_to",
  "timezone",
  "position",
  "game",
  "tournament_id",
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

export function workspaceHref(
  workspace: Workspace,
  section: string = DEFAULT_SECTIONS[workspace],
): string {
  if (workspace === "today") return "/today";
  return `/${workspace}/${section}`;
}

export function parseRoute(pathname: string): AppRoute | null {
  const parts = pathname.split("/").filter(Boolean);
  if (!parts.length) return null;
  const workspace = parts[0] as Workspace;
  if (!WORKSPACES.includes(workspace)) return null;
  if (workspace === "hands" && parts[1] === "replay") {
    const handId = Number(parts[2]);
    if (!parts[2] || !Number.isInteger(handId) || handId <= 0) return null;
    return {
      workspace: "hands",
      section: "list",
      pathname: workspaceHref("hands", "list"),
      handId,
    };
  }
  const section = parts[1] || DEFAULT_SECTIONS[workspace];
  // Keep existing shared links and bookmarks working after settings moved out.
  if (workspace === "data" && section === "preferences") {
    return {
      workspace: "settings",
      section: "preferences",
      pathname: workspaceHref("settings", "preferences"),
    };
  }
  if (!(VALID_SECTIONS[workspace] as ReadonlySet<string>).has(section)) {
    return null;
  }
  return { workspace, section, pathname: workspaceHref(workspace, section) };
}

export function isSafeReturnPath(pathname: string): boolean {
  if (!pathname.startsWith("/") || pathname.startsWith("//")) return false;
  const route = parseRoute(pathname);
  return Boolean(route) && route!.workspace !== "hands";
}

export function filterToSearchParams(filter: Filter): URLSearchParams {
  const params = new URLSearchParams();
  const entries = Object.entries(filter);
  if (!entries.some(([key, value]) => key === "currency" && value)) {
    params.set("currency", "USD");
  }
  for (const [key, value] of entries) {
    if (value === undefined || value === null || value === "") continue;
    if (NUMBER_FILTER_KEYS.has(key)) params.set(key, String(value));
    else if (BOOLEAN_FILTER_KEYS.has(key)) params.set(key, String(value));
    else if (STRING_FILTER_KEYS.has(key)) params.set(key, String(value));
  }
  return params;
}

export function filterFromSearchParams(
  params: URLSearchParams,
): Partial<Filter> {
  const filter: Record<string, string | number | boolean> = {};
  for (const [key, value] of params.entries()) {
    if (!value) continue;
    if (NUMBER_FILTER_KEYS.has(key)) {
      const parsed = Number(value);
      if (Number.isFinite(parsed) && parsed >= 0) filter[key] = parsed;
    } else if (BOOLEAN_FILTER_KEYS.has(key)) {
      if (value === "true") filter[key] = true;
      if (value === "false") filter[key] = false;
    } else if (STRING_FILTER_KEYS.has(key)) {
      filter[key] = value;
    }
  }
  return filter as Partial<Filter>;
}

export function routeSearch(
  filter: Filter,
  extra: Record<string, string | number | undefined> = {},
): string {
  const params = filterToSearchParams(filter);
  const allowedExtras = new Set([
    "dataset",
    "pack",
    "node",
    "tab",
    "step",
    "from",
  ]);
  for (const [key, value] of Object.entries(extra)) {
    if (!allowedExtras.has(key)) continue;
    if (value === undefined || value === null || value === "") {
      params.delete(key);
      continue;
    }
    params.set(key, String(value));
  }
  const search = params.toString();
  return search ? `?${search}` : "";
}
