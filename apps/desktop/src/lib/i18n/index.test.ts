import { test } from "node:test";
import assert from "node:assert/strict";
import { resolveLanguage, translate } from "./index.ts";

test("the language follows the setting, or the system when asked", () => {
  assert.equal(resolveLanguage("en", ["pt-BR"]), "en");
  assert.equal(resolveLanguage("pt-BR", ["en-US"]), "pt-BR");
  assert.equal(resolveLanguage("system", ["pt-PT", "en"]), "pt-BR");
  assert.equal(resolveLanguage("system", ["de-DE"]), "en");
  assert.equal(resolveLanguage("system", []), "en");
});

test("fills placeholders and falls back to English, then to the key", () => {
  const en = { hello: "Hello {name}", only_en: "English only" };
  const pt = { hello: "Olá {name}" };
  assert.equal(translate(pt, en, "hello", { name: "Luis" }), "Olá Luis");
  assert.equal(translate(pt, en, "only_en"), "English only");
  assert.equal(translate(pt, en, "nowhere"), "nowhere");
  assert.equal(translate(en, en, "hello", {}), "Hello {name}", "an unfilled placeholder stays visible");
  assert.equal(translate(en, en, "hello", { name: 3 }), "Hello 3");
});
