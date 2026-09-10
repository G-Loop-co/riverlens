import fs from "node:fs";
import * as OpenCC from "opencc-js";

// Generate a checked-in Simplified Chinese catalog, never convert user content.
const traditional = JSON.parse(
  fs.readFileSync("src/locales/zh-TW.json", "utf8"),
);
const convert = OpenCC.Converter({ from: "tw", to: "cn" });
const glossary = [
  ["资料库", "数据库"],
  ["资料夹", "文件夹"],
  ["资料", "数据"],
  ["汇入", "导入"],
  ["汇出", "导出"],
  ["档案", "文件"],
  ["本机", "本地"],
  ["储存", "保存"],
  ["设定", "设置"],
  ["帐本", "账本"],
  ["介面", "界面"],
];
const simplified = Object.fromEntries(
  Object.entries(traditional).map(([key, value]) => [
    key,
    glossary.reduce(
      (text, [from, to]) => text.replaceAll(from, to),
      convert(value),
    ),
  ]),
);
fs.writeFileSync(
  "src/locales/zh-CN.json",
  JSON.stringify(simplified, null, 2) + "\n",
);
