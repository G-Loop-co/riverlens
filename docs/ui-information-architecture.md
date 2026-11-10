# RiverLens UI information architecture

Updated: 2026-11-09

The interface now has six primary workspaces instead of nine flat pages. The app uses `HashRouter` so refresh, browser history and hand-history deep links work without a server-side route handler.

## Routes

| Route                          | Workspace      | Purpose                                                      |
| ------------------------------ | -------------- | ------------------------------------------------------------ |
| `/today`                       | Overview       | Results, trends, core statistics and next actions.           |
| `/hands/list`                  | Hand review    | Hand library, annotations and exports.                       |
| `/hands/matrix`                | Hand review    | Observed starting-hand matrix.                               |
| `/hands/review`                | Hand review    | Situation-focused review entry points.                       |
| `/hands/replay/:handId?step=n` | Replayer modal | Route-backed replay over the retained hand-review workspace. |
| `/learn/spots`                 | Study          | Spot Explorer and current study cohort.                      |
| `/learn/progress`              | Study          | Practice and index coverage.                                 |
| `/learn/leaks`                 | Study          | Frequency targets and snapshots.                             |
| `/learn/strategy`              | Study          | GTO/reference comparison.                                    |
| `/learn/range`                 | Study          | Range drills.                                                |
| `/learn/trainer`               | Study          | Daily training.                                              |
| `/research/stats`              | MDA research   | Population statistics.                                       |
| `/research/wizard`             | MDA research   | Disciplined research flow.                                   |
| `/research/candidates`         | MDA research   | Candidate hypotheses.                                        |
| `/research/evidence`           | MDA research   | Anonymous hand evidence.                                     |
| `/research/ai`                 | MDA research   | Evidence-based population analysis.                          |
| `/research/import`             | MDA research   | Import jobs and quarantine.                                  |
| `/coach/situation`             | AI coach       | Prompt-first analysis with the current context.              |
| `/coach/chat`                  | AI coach       | Situation explorer.                                          |
| `/coach/library`               | AI coach       | Drafts, attempts and evidence.                               |
| `/coach/connections`           | AI coach       | Models, keys and external connections.                       |
| `/data/import`                 | Data center    | Import center.                                               |
| `/data/sources`                | Data center    | Sources, backup, restore and engine controls.                |
| `/data/preferences`            | Data center    | Theme and language.                                          |

## URL contract

- Personal report filters use the existing `Filter` names as query parameters and default to `currency=USD`.
- Dataset is carried as `dataset`; study strategy context as `pack` and `node`; replay step as `step`.
- Safe origin workspaces may be carried as `from` when opening the replay modal.
- AI prompts, raw hand text, player names, notes and tags never enter the URL.
- Unknown routes redirect to `/today`. Tabs use the WAI-ARIA tab pattern with arrow-key navigation.

## UI rules

- Import actions appear on Today and the hand list, not on every page.
- Research setup actions are compact and backup/restore are grouped as dataset actions.
- Data-center tabs separate import, safety operations and interface preferences.
- Base body text is 15px, secondary text is 13px and captions are at least 12px.
- Route errors stay local to the active workspace through route error boundaries.
