import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import en from "./en.ts";
import ptBR from "./pt-BR.ts";
import { METRICS } from "../metrics.ts";

const src = fileURLToPath(new URL("../../", import.meta.url));

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    return statSync(p).isDirectory() ? files(p) : /\.(svelte|ts)$/.test(name) && !/\.test\.ts$/.test(name) ? [p] : [];
  });
}

test("Portuguese has exactly the English keys", () => {
  assert.deepEqual(Object.keys(ptBR).sort(), Object.keys(en).sort());
});

test("no translation is empty, and placeholders match the English ones", () => {
  const holes = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
  for (const [key, text] of Object.entries(en)) {
    assert.ok(text.trim() && ptBR[key]?.trim(), `${key} is empty`);
    assert.deepEqual(holes(ptBR[key]), holes(text), `${key} must use the same {placeholders}`);
  }
});

test("every metric has a name in both languages", () => {
  for (const m of METRICS) {
    assert.ok(en[`metric.${m.key}`], `English name of ${m.key}`);
    assert.ok(ptBR[`metric.${m.key}`], `Portuguese name of ${m.key}`);
  }
});

test("every key the code asks for exists", () => {
  const used = new Set<string>();
  for (const file of files(src)) {
    const text = readFileSync(file, "utf8");
    for (const m of text.matchAll(/\bt\(\s*["'`]([a-z0-9_.]+)["'`]/g)) used.add(m[1]);
  }
  assert.ok(used.size > 10, "found the keys the code uses");
  const missing = [...used].filter((k) => !(k in en));
  assert.deepEqual(missing, [], "keys used in the code but missing from the dictionary");
});

// Turned on in the branch that adds the last views: until then the History, Alerts and Settings strings wait for them.
test("every key in the dictionary is used somewhere", { skip: "the remaining views have not landed yet" }, () => {
  const all = files(src).filter((f) => !f.includes("/i18n/")).map((f) => readFileSync(f, "utf8")).join("\n");
  // Keys built at run time (`t(\`nav.${id}\`)`) are matched by their prefix.
  const unused = Object.keys(en).filter((k) => {
    if (all.includes(`"${k}"`) || all.includes(`'${k}'`)) return false;
    const prefix = k.slice(0, k.lastIndexOf(".") + 1);
    return !all.includes("`" + prefix + "$");
  });
  assert.deepEqual(unused, [], "dictionary keys nothing uses");
});
