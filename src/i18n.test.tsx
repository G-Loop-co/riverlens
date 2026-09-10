import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import fs from "node:fs";
import ts from "typescript";
import i18n, { changeLanguage, LANGUAGE_KEY, supportedLanguage } from "./i18n";
import traditional from "./locales/zh-TW.json";
import simplified from "./locales/zh-CN.json";
import english from "./locales/en.json";
import { AdvancedFilters } from "./components/Filters";
import { diagnosticMessage } from "./diagnostics";

afterEach(async () => {
  vi.unstubAllGlobals();
  await i18n.changeLanguage("zh-TW");
});

describe("localized interface boundaries", () => {
  it("provides matching keys and interpolation variables in all three catalogs", () => {
    const variables = (value: string) =>
      [...value.matchAll(/{{(.*?)}}/g)].map((m) => m[1]).sort();
    for (const catalog of [simplified, english]) {
      expect(Object.keys(catalog).sort()).toEqual(
        Object.keys(traditional).sort(),
      );
      for (const key of Object.keys(
        traditional,
      ) as (keyof typeof traditional)[]) {
        expect(variables(catalog[key]), key).toEqual(
          variables(traditional[key]),
        );
        expect(catalog[key].trim(), key).not.toBe("");
      }
    }
    expect(
      Object.values(traditional).filter((v) => /[嘅喺睇唔呢]/.test(v)),
    ).toEqual([]);
    expect(
      Object.values(english).filter((v) => /\p{Script=Han}/u.test(v)),
    ).toEqual([]);
  });

  it("covers literal UI translation keys and backend statistic definitions", () => {
    const files = [
      "src/App.tsx",
      "src/api.ts",
      ...fs
        .readdirSync("src/components")
        .filter((f) => f.endsWith(".tsx"))
        .map((f) => `src/components/${f}`),
    ];
    const missing: string[] = [];
    for (const file of files) {
      const source = ts.createSourceFile(
        file,
        fs.readFileSync(file, "utf8"),
        ts.ScriptTarget.Latest,
        true,
        ts.ScriptKind.TSX,
      );
      function visit(node: ts.Node) {
        if (
          ts.isCallExpression(node) &&
          /^(?:i18n\.)?t$/.test(node.expression.getText(source)) &&
          node.arguments[0] &&
          ts.isStringLiteral(node.arguments[0])
        ) {
          const key = node.arguments[0].text;
          if (!(key in traditional)) missing.push(`${file}: ${key}`);
        }
        ts.forEachChild(node, visit);
      }
      visit(source);
    }
    const definitions = fs
      .readFileSync("crates/poker-core/src/model.rs", "utf8")
      .split("pub const STAT_DEFINITIONS")[1];
    for (const match of definitions.matchAll(
      /"([^"\n]*[\p{Script=Han}][^"\n]*)"/gu,
    )) {
      if (!(match[1] in traditional)) missing.push(match[1]);
    }
    expect(missing).toEqual([]);
  });

  it("keeps canonical filter values unchanged when labels change language", async () => {
    for (const language of ["zh-TW", "zh-CN", "en"] as const) {
      await i18n.changeLanguage(language);
      const html = renderToStaticMarkup(
        <AdvancedFilters
          filter={{
            pot_type: "limped",
            street: "turn",
            action: "call",
            paired: false,
          }}
          update={() => {}}
        />,
      );
      for (const value of ["limped", "turn", "call", "false"]) {
        expect(html).toContain(`value="${value}" selected=""`);
      }
    }
  });

  it("persists only the language preference and rejects unsupported preferences", async () => {
    const values = new Map([["riverlens-filter", '{"position":"BTN"}']]);
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
    });
    await changeLanguage("zh-CN");
    expect(values.get(LANGUAGE_KEY)).toBe("zh-CN");
    expect(values.get("riverlens-filter")).toBe('{"position":"BTN"}');
    expect(supportedLanguage(values.get(LANGUAGE_KEY)!)).toBe("zh-CN");
    expect(supportedLanguage("unsupported")).toBe("zh-TW");
  });

  it("handles English counts and preserves interpolated user content", async () => {
    await i18n.changeLanguage("en");
    expect(i18n.t("{{v0}} 個工作", { count: 1, v0: 1 })).toBe("1 job");
    expect(i18n.t("{{v0}} 個工作", { count: 2, v0: 2 })).toBe("2 jobs");
    expect(i18n.t("{{v0}} 手資料異常已排除", { count: 1, v0: 1 })).toBe(
      "1 hand excluded for data issues",
    );
    const path = "/Users/example/我的牌譜/<draft>.db";
    const html = renderToStaticMarkup(
      <p>{i18n.t("備份已儲存：{{v0}}", { v0: path })}</p>,
    );
    expect(html).toContain("我的牌譜/&lt;draft&gt;.db");
  });

  it("localizes known diagnostics without changing numeric evidence or unknown text", async () => {
    const original =
      "Action ledger 1.14 differs from total pot 1.32 (delta -0.18)";
    await i18n.changeLanguage("zh-CN");
    expect(diagnosticMessage(original)).toBe(
      "行动账本 1.14 与总底池 1.32 不一致（差额 -0.18）",
    );
    await i18n.changeLanguage("en");
    expect(diagnosticMessage(original)).toBe(original);
    expect(diagnosticMessage("unrecognized diagnostic /local/原文.txt")).toBe(
      "unrecognized diagnostic /local/原文.txt",
    );
  });
});
