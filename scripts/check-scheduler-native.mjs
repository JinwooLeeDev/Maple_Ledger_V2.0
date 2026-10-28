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
  const initial=await view();
  assert.ok(initial.characters.every(c=>c.character.identity.ocid.startsWith("ui-")));
  assert.equal(initial.scheduler.entries.length,1);
  assert.equal(initial.scheduler.entries[0].content_name,"\uD53C\uC5D0\uB974");
  if (!await evaluate("document.querySelector('.boss-page') !== null")) await click("\uBCF4\uC2A4 \uAD00\uB9AC");
  await ready();
  await until(`document.querySelector('[aria-label="\uC2A4\uCF00\uC904\uB7EC \uB4F1\uB85D \uBCF4\uC2A4 \uBAA9\uB85D"]') !== null`);
  const list=await evaluate(`document.querySelector('[aria-label="\uC2A4\uCF00\uC904\uB7EC \uB4F1\uB85D \uBCF4\uC2A4 \uBAA9\uB85D"]').innerText`);
  assert.ok(list.includes("chaos"));
  assert.ok(list.includes("4,080,000"));
  assert.ok(!list.includes("\uB9BC\uBCF4"));
  assert.equal(initial.history.length,1);
  assert.equal(initial.total.amount,"3");
  const api=await evaluate(`window.__TAURI_INTERNALS__.invoke('check_boss_api',{ocid:'ui-one',week:${initial.week.start_day},selectionRevision:${initial.selection_revision}}).catch(e=>e.code)`);
  assert.equal(api,"offline_check");
  assert.equal((await view()).scheduler.id,initial.scheduler.id);
  console.log("PASS 7-E native: registered-only list, raw difficulty, price total, separate manual history, offline failure persistence; restart="+process.argv.includes("--restore"));
} finally { socket.close(); }
