import type { Filter } from "./types";

export interface DecisionRef {
  profile: string;
  hand_id: string;
  seq: number;
  version: string;
}
export interface LineAction {
  street: string;
  actor: string;
  action: string;
}
export interface SpotDefinition {
  scenario?: string;
  preflop_path?: string;
  street?: string;
  position?: string;
  opponent?: string;
  role?: string;
  facing?: string;
  pot_type?: string;
  effective_min?: number;
  effective_max?: number;
  texture?: string;
  paired?: boolean;
  high_card?: string;
  facing_min?: number;
  facing_max?: number;
  line: LineAction[];
}
export interface Decision {
  reference: DecisionRef;
  street: string;
  position: string;
  opponent: string | null;
  role: string | null;
  facing: string;
  pot_type: string;
  effective_bb: number | null;
  action: string;
  size_bb: number | null;
  bet_pct: number | null;
  facing_pct: number | null;
  preflop_path: string;
  line: LineAction[];
  legal: string[];
  board: string[];
  hand_class: string;
  cards: string[];
  player_count: number;
  stacks_bb: Record<string, number>;
  sb_bb: number;
  bb: string;
  currency: string;
  game: string;
  special: boolean;
  pot_before: string;
  to_call: string;
}
export interface StudyQuery {
  filter: Filter;
  spot: SpotDefinition;
  before?: number | null;
  limit?: number;
  strategy?: { pack: string; node: string; matched_only: boolean };
}
export interface Coverage {
  total: number;
  indexed: number;
  complete: boolean;
  version: string;
}
export interface SpotRow {
  id: number;
  hand: number;
  decision: Decision;
  played_at: number;
  net_bb: number;
}
export interface SpotReport {
  opportunities: number;
  hands: number;
  net_bb: number;
  actions: Record<string, number>;
  rows: SpotRow[];
  next_cursor: number | null;
  coverage: Coverage;
}
export interface StudyItem<T> {
  id: string;
  data: T;
}
export interface SavedSpot {
  name: string;
  spot: SpotDefinition;
}
export interface Benchmark extends SavedSpot {
  action: string;
  low: number;
  high: number;
  min_samples: number;
  note: string;
}
export interface LeakRow {
  name_key?: string | null;
  id: string;
  name: string;
  spot?: SpotDefinition;
  source: "custom" | "gto" | "observation";
  note: string;
  action: string;
  low: number | null;
  high: number | null;
  actual: number | null;
  gap: number | null;
  interval: [number, number] | null;
  opportunities: number;
  hits: number;
  enough: boolean;
  priority: number | null;
  node?: string;
  pack?: string;
}
export interface LeakReport {
  rows: LeakRow[];
  coverage: Coverage;
}
export interface PackSummary {
  id: string;
  name: string;
  source: Record<string, unknown>;
  files: number;
  nodes: number;
  ready_cells: number;
  reference_cells: number;
  imported_at: number;
}
export interface StrategyAction {
  code: string;
  label: string;
  kind: string;
  size_bb: number | null;
}
export interface NodeSummary {
  id: string;
  hero: string;
  path: string;
  actions: StrategyAction[];
  ready_cells: number;
  reference_cells: number;
}
export interface StrategyNode extends NodeSummary {
  source_url: string;
  frequencies: Record<string, Record<string, number>>;
  issues: Record<string, string>;
  reach: Record<string, number>;
}
export interface StrategyCell {
  hand: string;
  strategy: Record<string, number> | null;
  issue: string | null;
  observed: {
    opportunities: number;
    matched: number;
    actions: Record<string, number>;
    matched_actions: Record<string, number>;
    expected: Record<string, number>;
  } | null;
}
export interface StrategyMatrix {
  node: StrategyNode;
  source: Record<string, unknown>;
  cells: StrategyCell[];
  reasons: Record<string, number>;
  coverage: Coverage;
}
export interface DecisionComparison {
  status: "matched" | "reference" | "unmatched";
  reasons: string[];
  decision: Decision;
  node?: StrategyNode;
  frequency?: Record<string, number> | null;
  observed_action?: string | null;
}
export type TrainingAnswer =
  | { mode: "action"; action: string; note: string }
  | { mode: "frequency"; frequencies: Record<string, number>; note: string };
export interface TrainingQuestion {
  reference?: DecisionRef;
  street: string;
  position: string;
  opponent?: string | null;
  role?: string | null;
  board: string[];
  cards: string[];
  hand_class: string;
  pot_before?: string;
  to_call?: string;
  line: LineAction[];
  preflop_path: string;
  legal?: string[];
  stacks_bb?: Record<string, number>;
  facing?: string;
  options?: StrategyAction[];
  theory?: boolean;
}
export interface TrainingFeedback {
  answer: TrainingAnswer;
  feedback: {
    chosen?: string;
    in_strategy?: boolean | null;
    differences?: Record<string, number>;
    max_gap?: number;
  };
  expected: Record<string, number> | null;
  observed: string | null;
  source_note: string;
  source: string;
  pack: string | null;
  reference: DecisionRef | null;
  replay_hand: number | null;
  rating: string | null;
  created: number;
}
export interface TrainingCurrent {
  id: string;
  complete: boolean;
  index: number;
  total: number;
  card_id?: string;
  question?: TrainingQuestion;
  source?: string;
  pack?: string | null;
  feedback?: TrainingFeedback | null;
  stale?: boolean;
}
export interface TrainingState {
  total: number;
  due: number;
  completed: number;
  active: string | null;
  recent: TrainingFeedback[];
}
export type StudyRequest =
  | { op: "explore"; query: StudyQuery }
  | { op: "items"; kind: "spot" | "benchmark" }
  | {
      op: "save_item";
      kind: "spot" | "benchmark";
      id?: string;
      data: SavedSpot | Benchmark;
    }
  | { op: "delete_item"; kind: "spot" | "benchmark"; id: string }
  | { op: "presets" }
  | { op: "hand_decisions"; hand: number }
  | { op: "packs" }
  | { op: "import_pack"; path: string }
  | { op: "nodes"; pack: string }
  | { op: "matrix"; pack: string; node: string; filter: Filter }
  | { op: "compare"; pack: string; reference: DecisionRef; node?: string }
  | { op: "leaks"; filter: Filter; pack?: string }
  | {
      op: "enqueue";
      references: DecisionRef[];
      pack?: string;
      node?: string;
      hand_class?: string;
    }
  | { op: "training_state" }
  | { op: "training_start"; limit: number }
  | { op: "training_current"; session: string }
  | {
      op: "training_answer";
      session: string;
      card: string;
      answer: TrainingAnswer;
    }
  | {
      op: "training_rate";
      session: string;
      card: string;
      rating: "repeat" | "advance" | "skip";
    }
  | { op: "resolve"; reference: DecisionRef };
