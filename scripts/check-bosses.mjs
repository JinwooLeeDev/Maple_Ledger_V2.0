import assert from "node:assert/strict";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createServer } from "vite";
import fs from "node:fs";

const server = await createServer({ server: { middlewareMode: true }, appType: "custom", logLevel: "silent" });
try {
  const { BossEditor, BossTotals, SchedulerList } = await server.ssrLoadModule("/src/features/bosses/BossPage.tsx");
  const { BossApiPanel } = await server.ssrLoadModule("/src/features/bosses/BossSupplements.tsx");
  const { previewIncome, currentWeekDay } = await server.ssrLoadModule("/src/shared/api/bosses.ts");
  assert.equal(previewIncome("4040000", "3"), "1346666");
  for (const party of ["0", "-1", "1.2", "2147483648"]) assert.equal(previewIncome("4040000", party), null);
  assert.equal(previewIncome(null, "1"), null);
  assert.equal(previewIncome("9223372036854775808", "1"), null);
  const boundary = Date.parse("2026-10-22T00:00:00+09:00");
  assert.equal(currentWeekDay(boundary) - currentWeekDay(boundary - 1), 7);
  const catalog = JSON.parse(fs.readFileSync("src-tauri/resources/bosses.json", "utf8"));
  const character = { identity: { ocid: "synthetic", account_id: "account", world_name: "월드" }, character_name: "합성 캐릭터", character_class: "합성 직업", character_level: 280 };
  const view = { character, week: { start_day: currentWeekDay(boundary), current_start: currentWeekDay(boundary), start_date: "2026-10-22", end_date: "2026-10-29", today: "2026-10-28" }, character_total: { amount: "1346666", unpriced: 1 }, total: { amount: "5387000000", unpriced: 2 } };
  const known = renderToStaticMarkup(createElement(BossEditor, { variants: catalog.filter(v => v.boss_id === "zakum"), view, plan: { enabled: true, variant_id: "zakum-chaos" }, disabled: false, onSave() {} }));
  assert.ok(known.includes("4,040,000 메소")); assert.ok(known.includes('value="1"')); assert.ok(known.includes("기록 저장"));
  const unknown = renderToStaticMarkup(createElement(BossEditor, { variants: catalog.filter(v => v.id === "limbo-extreme"), view, disabled: false, onSave() {} }));
  assert.ok(unknown.includes("미산정")); assert.ok(!unknown.includes("0 메소"));
  const totals = renderToStaticMarkup(createElement(BossTotals, { view }));
  assert.ok(totals.includes("1,346,666 메소")); assert.ok(totals.includes("5,387,000,000 메소")); assert.ok(totals.includes("미산정 2건"));
  const enhanced = { ...view, catalog, managed: true, records: [], plans: [{boss_id: "zakum", variant_id:"zakum-chaos", enabled: true}], observation: {id: 1, query_date: "2026-10-28", observed_at: 1, entries: [{ content_name: "자쿰", difficulty: "chaos", cycle: "bossWeekly", completed: true, variant_id: "zakum-chaos" }, { content_name: "UNKNOWN", difficulty: "hard", cycle: "bossWeekly", completed: true, variant_id: null }]}};
  const scheduler = { ...enhanced.observation, entries: enhanced.observation.entries.map(b => ({...b,registered:true})) };
  const listed = renderToStaticMarkup(createElement(SchedulerList, {view:{...enhanced,scheduler},disabled:false,onSave(){}}));
  assert.ok(listed.includes("스케줄러 등록 보스 2개"));
  assert.ok(listed.includes("자쿰 · chaos"));
  assert.ok(listed.includes("UNKNOWN · hard"));
  assert.ok(listed.includes("등록 결정석 가격 합계: 4,040,000 메소"));
  assert.ok(listed.includes("예상 주간 개인 결정석 수익: 4,040,000 메소"));
  assert.ok(listed.includes("가격 미확인 1건"));
  assert.ok(listed.includes("기록된 클리어: 미완료"));
  const partyList=renderToStaticMarkup(createElement(SchedulerList,{view:{...enhanced,scheduler,records:[{variant_id:"zakum-chaos",clear_group:"zakum-chaos",party:3,completed:true,personal_income:"7",solo_price:"21",revision:1}]},disabled:false,onSave(){}}));
  assert.ok(partyList.includes("예상 주간 개인 결정석 수익: 1,346,666 메소"));
  assert.ok(partyList.includes("기록 개인 수익: 7 메소"));
  assert.ok(!known.includes("가격 직접 입력")); assert.ok(!known.includes("수동 가격"));
  const api = renderToStaticMarkup(createElement(BossApiPanel, {view:enhanced,disabled:false,mutate(){}}));
  assert.ok(api.includes("API 완료 상태 확인")); assert.ok(api.includes("선택 기록 반영")); assert.ok(api.includes("매핑 미확인"));
  assert.ok(api.includes('type="date" required=""')); assert.ok(api.includes('value=""'));
  const conflict = renderToStaticMarkup(createElement(BossApiPanel, {view:{...enhanced,records:[{clear_group:"zakum-chaos",completed:false}]},disabled:false,mutate(){}}));
  assert.ok(conflict.includes("기존 기록 유지")); assert.ok(!conflict.includes("선택 기록 반영"));
  console.log("PASS: read-only prices, selected API rows and legacy conflict; integer preview, invalid/unknown prices, KST boundary, boss inputs and distinct unpriced totals");
} finally { await server.close(); }

