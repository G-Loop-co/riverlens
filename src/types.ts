export type Money = string;
export interface Profile {
  id: string;
  name: string;
  brand: string;
  hero: string;
  timezone: string;
}
export interface Filter {
  source_date?: string;
  profile?: string;
  date_from?: string;
  date_to?: string;
  timezone?: string;
  position?: string;
  game?: string;
  stakes?: string;
  currency?: string;
  player_count?: number;
  pot_type?: string;
  hand_class?: string;
  texture?: string;
  paired?: boolean;
  high_card?: string;
  showdown?: boolean;
  stack_min?: number;
  stack_max?: number;
  effective_min?: number;
  effective_max?: number;
  flop_players?: number;
  stat?: string;
  stat_mode?: string;
  cue?: string;
  street?: string;
  action?: string;
  bet_min?: number;
  bet_max?: number;
  tag?: string;
  reviewed?: boolean;
  search?: string;
  status?: string;
  ev_status?: string;
  session?: string;
  result?: string;
}
export interface Stat {
  id: string;
  label: string;
  definition: string;
  numerator: number;
  opportunities: number;
  value: number | null;
}
export interface Group {
  key: string;
  hands: number;
  net: Money;
  net_bb: number;
  bb100: number;
  sd: Money;
  nsd: Money;
  adjusted: Money;
}
export interface Report {
  groups_truncated: boolean;
  hands: number;
  net: Money | null;
  currency: string;
  net_bb: number;
  bb100: number;
  sd: Money;
  nsd: Money;
  adjusted: Money;
  stats: Stat[];
  groups: Group[];
  coverage: Record<string, number>;
  ev: {
    complete: number;
    pending: number;
    excluded: number;
    reasons: Record<string, number>;
  };
  stats_version: string;
}
export interface HandRow {
  id: number;
  hand_id: string;
  played_at: number;
  date: string;
  position: string;
  hand_class: string;
  net: Money;
  net_bb: number;
  stakes: string;
  pot_type: string;
  game: string;
  status: string;
  ev_status: string;
  reviewed: boolean;
  sort_value: number;
}
export interface Cursor {
  value: number;
  id: number;
}
export interface HandPage {
  rows: HandRow[];
  next_cursor: Cursor | null;
}
export interface Player {
  seat: number;
  name: string;
  hero: boolean;
  stack: Money;
  cards: string[];
  position: string;
}
export interface Action {
  seq: number;
  street: string;
  runout: number;
  actor: number | null;
  kind: string;
  amount: Money;
  to: Money;
  all_in: boolean;
  cards: string[];
  pot_after: Money;
}
export interface Hand {
  id: string;
  profile: string;
  brand: string;
  played_at: number;
  local_time: string;
  timezone: string;
  table_name: string;
  max_seats: number;
  player_count: number;
  currency: string;
  game: string;
  sb: Money;
  bb: Money;
  button: number;
  hero_seat: number;
  position: string;
  hand_class: string;
  players: Player[];
  actions: Action[];
  boards: string[][];
  pots: { amount: Money; eligible: number[] }[];
  invested: Money;
  returned: Money;
  collected: Money;
  cashout: Money;
  cashout_risk: Money;
  net: Money;
  total_pot: Money;
  fees: Record<string, Money>;
  showdown: boolean;
  saw_flop: boolean;
  pot_type: string;
  status: string;
  issues: { code: string; message: string; line: number | null }[];
  ev_status: string;
  ev_reason: string | null;
  equity: number | null;
  adjusted_net: Money | null;
  raw: string;
}
export interface Annotation {
  note: string;
  tags: string[];
  reviewed: boolean;
}
export interface HandDetail {
  id: number;
  hand: Hand;
  annotation: Annotation;
}
export interface MatrixCell {
  hand: string;
  hands: number;
  net: Money;
  net_bb: number;
  bb100: number;
  numerator: number;
  opportunities: number;
}
export interface Job {
  id: string;
  profile: Profile;
  paths: string[];
  state: string;
  scanned: number;
  inserted: number;
  duplicates: number;
  quarantined: number;
  conflicts: number;
  files_done: number;
  files_total: number;
  current_file: string;
  message: string | null;
  started_at: number;
  elapsed_ms: number;
}
export interface ImportIssue {
  id: number;
  job: string;
  file: string;
  hand_id: string | null;
  code: string;
  message: string;
  hand_row: number | null;
}
export interface SavedFilter {
  id: number;
  name: string;
  filter: Filter;
}
export interface Health {
  study?: import("./study-types").Coverage;
  study_running?: boolean;
  study_paused?: boolean;
  study_error?: string | null;
  equity_error: string | null;
  version: string;
  parser: string;
  stats: string;
  database: string;
  import_active: boolean;
  equity_running: boolean;
  equity_paused: boolean;
  schema: number;
  offline: boolean;
}
export type Request =
  | { op: "study"; request: import("./study-types").StudyRequest }
  | { op: "study_control"; paused: boolean }
  | { op: "overview"; filter: Filter; group?: string }
  | {
      op: "hands";
      filter: Filter;
      sort?: string;
      cursor?: Cursor | null;
      limit?: number;
    }
  | { op: "hand"; id: number }
  | { op: "matrix"; filter: Filter; stat: string }
  | { op: "save_annotation"; id: number; annotation: Annotation }
  | {
      op: "profiles" | "jobs" | "saved_filters" | "rebuild" | "health";
    }
  | { op: "issues"; before?: number | null }
  | { op: "save_profile"; profile: Profile }
  | { op: "start_import"; paths: string[]; profile: Profile }
  | { op: "cancel_import" | "resume_import"; id: string }
  | { op: "issue_raw" | "delete_filter"; id: number }
  | { op: "save_filter"; name: string; filter: Filter }
  | { op: "backup" | "restore"; path: string }
  | {
      op: "export";
      path: string;
      format: string;
      filter: Filter;
      selected: number[];
    }
  | { op: "equity_control"; paused: boolean };
