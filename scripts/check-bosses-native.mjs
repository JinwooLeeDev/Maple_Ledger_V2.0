// Explicit development-only verification against an already running isolated Tauri app.
// Start with MAPLE_LEDGER_CHECK_DIR, MAPLE_LEDGER_OFFLINE_CHECK=1 and WebView2 CDP port 9222.
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const targets = await (await fetch("http://127.0.0.1:9222/json")).json();
const target = targets.find(t => t.type === "page" && t.url === "http://127.0.0.1:1420/");
assert.ok(target, "Isolated Tauri development page required");
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
let id = 0;
const pending = new Map();
socket.onmessage = e => { const message = JSON.parse(e.data); if (message.id) { const p = pending.get(message.id); pending.delete(message.id); message.error ? p?.reject(message.error) : p?.resolve(message.result); } };
function cdp(method, params = {}) { return new Promise((resolve, reject) => { const next = ++id; pending.set(next, {resolve, reject}); socket.send(JSON.stringify({id: next, method, params})); }); }
async function evaluate(expression) {
  const r = await cdp("Runtime.evaluate", {expression, awaitPromise: true, returnByValue: true});
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.text + " " + r.result?.description);
  return r.result.value;
}
const pause = () => new Promise(r => setTimeout(r, 120));
async function until(expression) { for (let n=0;n<70;n++) { if (await evaluate(expression)) return; await pause(); } throw new Error(`Timed out: ${expression}`); }
async function click(text, scope="document") { await evaluate(`(() => { const b=[...${scope}.querySelectorAll('button')].find(b => b.textContent.trim() === ${JSON.stringify(text)}); if (!b || b.disabled) throw Error('Button missing/disabled: '+${JSON.stringify(text)}); b.click(); })()`); await pause(); }
async function fill(selector, value, kind="input") { await evaluate(`(() => { const e=document.querySelector(${JSON.stringify(selector)}); if (!e) throw Error('Input missing'); Object.getOwnPropertyDescriptor(${kind === "select" ? "HTMLSelectElement" : "HTMLInputElement"}.prototype,'value').set.call(e,${JSON.stringify(value)}); e.dispatchEvent(new Event(${JSON.stringify(kind === "select" ? "change" : "input")},{bubbles:true})); })()`); await pause(); }
async function view() { return evaluate(`window.__TAURI_INTERNALS__.invoke('get_boss_view',{ocid:'ui-one',week:null})`); }
async function ready() { await until(`![...document.querySelectorAll('button')].some(b=>b.textContent==='기록 새로고침' && b.disabled)`); }
try {
  assert.ok((await evaluate("document.body.innerText")).includes("테스트 본캐"), "Never run against user data");
  // Read-only extra safeguard: this fixture always has synthetic OCIDs.
  const initial = await view();
  assert.ok(initial.characters.every(c => c.character.identity.ocid.startsWith("ui-")));
  await click("보스 관리");
  await until("document.body.innerText.includes('주간 보스 관리')");
  assert.ok(!("prices" in await view()));
  assert.ok(!(await evaluate("document.body.innerText")).includes("수동 가격"));
  if (process.argv.includes("--restore")) {
    const restored=await view();assert.equal(restored.total.amount,"1360005");assert.equal(restored.history.length,5);
    assert.equal(restored.records.find(r=>r.variant_id==='zakum-chaos').solo_price,"10");
    assert.ok(restored.records.find(r=>r.variant_id==='pierre-chaos').observation_id);
    console.log("PASS 7-D restart: legacy snapshot, API provenance, plans, 5 revisions and total restored");
  } else {
    const past=`[...document.querySelectorAll('details')].find(d=>d.querySelector('summary')?.textContent.includes('현재 관리 대상 외 기록'))`;
    assert.equal(await evaluate(`${past}.open`),false);
    const api=`[...document.querySelectorAll('details')].find(d=>d.querySelector('summary')?.textContent.includes('API 완료 상태'))`;
    await evaluate(`${api}.open=true`);
    assert.ok((await evaluate(`${api}.innerText`)).includes("스케줄러 등록"));
    assert.ok((await evaluate(`${api}.innerText`)).includes("스케줄러 미등록"));
    const unknown=`[...document.querySelectorAll('form.boss-editor')].find(f=>f.textContent.includes('API 완료') && f.textContent.includes('림보'))`;
    assert.ok((await evaluate(`${unknown}.innerText`)).includes("내장 가격 또는 적용 시점이 미확인"));
    assert.equal(await evaluate(`${unknown}.querySelector('button').disabled`),true);
    const row=`[...document.querySelectorAll('form.boss-editor')].find(f=>f.textContent.includes('API 완료') && f.textContent.includes('피에르'))`;
    await evaluate(`${row}.querySelector('input[type=checkbox]').click()`);
    await fill('form.boss-editor input[type=number]',"3");
    const today=(await view()).week.today;
    await evaluate(`(() => { const e=${row}.querySelector('input[type=date]'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(e,${JSON.stringify(today)}); e.dispatchEvent(new Event('input',{bubbles:true})); })()`);await pause();
    assert.ok((await evaluate(`${row}.innerText`)).includes("1,360,000 메소"));
    await click("선택 기록 반영",row);await ready();assert.equal((await view()).total.amount,"1360003");
    const editor=`document.querySelector('form[aria-label="피에르 클리어 기록"]')`;
    await evaluate(`${editor}.querySelector('input[type=checkbox]').click()`);await pause();await click("기록 저장",editor);await ready();assert.equal((await view()).total.amount,"3");
    await evaluate(`${editor}.querySelector('input[type=checkbox]').click()`);await pause();await click("기록 저장",editor);await ready();assert.equal((await view()).total.amount,"1360003");
    await evaluate(`${past}.open=true`);
    await fill('form[aria-label="자쿰 클리어 기록"] input[type=number]',"2");
    await click("기록 저장",`document.querySelector('form[aria-label="자쿰 클리어 기록"]')`);await ready();
    assert.equal((await view()).total.amount,"1360005");assert.equal((await view()).history.length,5);
    await click("API 완료 상태 확인");await until("document.body.innerText.includes('오프라인 검증 모드')");assert.equal((await view()).total.amount,"1360005");
    const denied=await evaluate(`window.__TAURI_INTERNALS__.invoke('save_boss_price',{input:{variant_id:'pierre-chaos',price:'1',reason:'synthetic',expected_revision:1}}).then(()=>false,()=>true)`);assert.equal(denied,true);
    const screenshot=await cdp("Page.captureScreenshot",{format:"png"});fs.writeFileSync(path.join(os.tmpdir(),"maple-ledger-7d-ui.png"),Buffer.from(screenshot.data,"base64"));
    console.log("PASS 7-D native: price controls/IPC removed, built-in price ignores old setting, legacy amount retained, non-managed history separated, API flags independent, unpriced blocked, API error preserves records, revision totals correct");
  }
} finally { socket.close(); }
