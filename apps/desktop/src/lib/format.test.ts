import { test } from "node:test";
import assert from "node:assert/strict";
import * as f from "./format.ts";

test("sizes count like macOS does: memory in 1024s, disks in 1000s", () => {
  assert.equal(f.bytes(34359738368), "32.0 GB", "32 GiB of RAM reads as 32 GB, like Activity Monitor");
  assert.equal(f.bytes(512 * 1000 ** 3, "en", "decimal"), "512 GB");
  assert.equal(f.bytes(1536), "1.50 KB");
  assert.equal(f.bytes(0), "0 B");
  assert.equal(f.bytes(1023), "1,023 B", "thousands are grouped by the locale");
  assert.equal(f.bytes(1023, "pt-BR"), "1.023 B");
  assert.equal(f.bytes(-5), "0 B");
});

test("Portuguese uses a decimal comma, English a point", () => {
  assert.equal(f.num(1234.5, 1, "en"), "1,234.5");
  assert.equal(f.num(1234.5, 1, "pt-BR"), "1.234,5");
  assert.equal(f.bytes(1536, "pt-BR"), "1,50 KB");
  assert.equal(f.percent(42.4, 0, "pt-BR"), "42%");
});

test("a missing or broken number is a dash, never NaN or undefined", () => {
  for (const fn of [f.num, f.percent, f.bytes, f.rate, f.temperature, f.watts, f.rpm, f.frequency]) {
    assert.equal(fn(null as unknown as number), "–");
    assert.equal(fn(undefined as unknown as number), "–");
    assert.equal(fn(Number.NaN), "–");
    assert.equal(fn(Number.POSITIVE_INFINITY), "–");
  }
  assert.equal(f.duration(null), "–");
  assert.equal(f.minutes(Number.NaN), "–");
});

test("rates, temperatures, power and clocks", () => {
  assert.equal(f.rate(2_500_000), "2.50 MB/s");
  assert.equal(f.temperature(61.4), "61 °C");
  assert.equal(f.watts(5.25), "5.3 W");
  assert.equal(f.watts(23.4), "23 W");
  assert.equal(f.rpm(2258), "2,258 rpm");
  assert.equal(f.frequency(3400), "3.40 GHz");
  assert.equal(f.frequency(800), "800 MHz");
});

test("durations use the two largest units", () => {
  assert.equal(f.duration(45), "45s");
  assert.equal(f.duration(185), "3m 05s");
  assert.equal(f.duration(7500), "2h 05m");
  assert.equal(f.duration(3 * 86400 + 4 * 3600), "3d 04h");
  assert.equal(f.duration(-1), "–");
  assert.equal(f.minutes(283), "4 h 43 min");
  assert.equal(f.minutes(45), "45 min");
});

test("a value carries its metric's unit", () => {
  assert.equal(f.value("percent", 12.6), "13%");
  assert.equal(f.value("bytes_per_second", 1000), "1.00 KB/s");
  assert.equal(f.value("celsius", 70), "70 °C");
  assert.equal(f.value("per_second", 3.7), "4/s");
});

test("the chip name drops trademarks and clocks, and survives a placeholder", () => {
  assert.equal(f.chipLabel("Apple M3 Pro"), "Apple M3 Pro");
  assert.equal(f.chipLabel("Intel(R) Core(TM) i9-9980HK CPU @ 2.40GHz"), "Intel Core i9-9980HK");
  assert.equal(f.chipLabel("Genuine Intel(R) 0000 @ 3.40GHz", 8), "Intel · 8 cores");
  assert.equal(f.chipLabel("", 0), "Intel");
});
