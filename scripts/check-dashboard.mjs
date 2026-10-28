import assert from "node:assert/strict";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createServer } from "vite";

// Synthetic SSR checks only; no app, credentials, DB, HTTP or browser install.
const server = await createServer({ server: { middlewareMode: true }, appType: "custom", logLevel: "silent" });
try {
  const { DashboardCards } = await server.ssrLoadModule("/src/features/dashboard/DashboardPage.tsx");
  const order = [0, 4, 2, 5, 1, 3];
  const cards = order.map((index, position) => ({
    character: { identity: { account_id: "synthetic-account", ocid: `synthetic-${index}`, world_name: "테스트 월드" }, character_name: `테스트-${index}`, character_class: "테스트 직업", character_level: 260 + index },
    is_main: position === 0, image_src: null, updated_at: null, error: position === 2 ? { code: "timeout", retry_after_seconds: null } : null, image_error: position === 3,
  }));
  const html = renderToStaticMarkup(createElement(DashboardCards, { cards }));
  assert.equal((html.match(/<article /g) ?? []).length, 6);
  assert.equal((html.match(/aria-label="본캐 캐릭터 카드"/g) ?? []).length, 1);
  assert.equal((html.match(/src="\/avatar-placeholder.svg"/g) ?? []).length, 6);
  let previous = -1;
  for (const index of order) {
    const position = html.indexOf(`<h3>테스트-${index}</h3>`);
    assert.ok(position > previous); previous = position;
  }
  assert.ok(html.includes("조회 시간이 초과"));
  assert.ok(html.includes("대체 이미지를 표시"));
  assert.ok(html.includes("Lv. 260") && html.includes("테스트 월드") && html.includes("테스트 직업"));
  assert.ok(!html.includes("synthetic-account") && !html.includes("synthetic-0"));
  const empty = renderToStaticMarkup(createElement(DashboardCards, { cards: [] }));
  assert.equal((empty.match(/<article /g) ?? []).length, 0);
  console.log("PASS: six cards, main first, stored order, metadata, fallback, independent error, empty list, hidden identifiers");
} finally { await server.close(); }
