import i18n from "./i18n";

// Translate known engine diagnostics only. Raw hand histories and user text never enter here.
const patterns: [RegExp, string][] = [
  [
    /^Action ledger ([-\d.]+) differs from total pot ([-\d.]+) \(delta ([-\d.]+)\)$/,
    "diagnostic.pot_mismatch",
  ],
  [
    /^Seat (\d+) (\w+) amount does not match current betting level$/,
    "diagnostic.action_mismatch",
  ],
  [/^Seat (\d+) action exceeds remaining stack$/, "diagnostic.over_stack"],
  [
    /^Seat (\d+) contribution exceeds stack or is negative$/,
    "diagnostic.stack_mismatch",
  ],
  [/^(\w+) board requires (\d+) cards$/, "diagnostic.board_length"],
  [/^Unknown actor on line (\d+)$/, "diagnostic.unknown_actor"],
  [/^Unrecognized settlement on line (\d+)$/, "diagnostic.unknown_money_line"],
  [/^Negative (\w+) contribution$/, "diagnostic.negative_delta"],
  [/^Unrecognized action: ([\s\S]+)$/, "diagnostic.unknown_action"],
];

export function diagnosticMessage(message: string): string {
  if (i18n.exists(message)) return i18n.t(message);
  for (const [pattern, key] of patterns) {
    const match = message.match(pattern);
    if (match)
      return i18n.t(
        key,
        Object.fromEntries(
          match.slice(1).map((value, index) => [`v${index}`, value]),
        ),
      );
  }
  return message;
}
