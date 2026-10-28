import { useState } from "react";
import { bossError, checkBossApi, effectivePrice, money, previewIncome, saveBossClear, type BossObservation, type BossView, type ObservedBoss } from "../../shared/api/bosses";

type Mutate = (operation: () => Promise<unknown>) => Promise<void>;

export function BossApiPanel({ view, disabled, mutate, onRefresh }: { view: BossView; disabled: boolean; mutate: Mutate; onRefresh?: () => void }) {
  const [loading, setLoading] = useState(false), [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BossObservation | null>(null);
  const [retryAt, setRetryAt] = useState(0);
  const observation = result ?? view.observation;
  const visible = observation?.entries.filter(b => b.variant_id !== null && (view.plans.some(p => p.enabled && p.variant_id === b.variant_id) || view.records.some(r => r.variant_id === b.variant_id))) ?? [];
  const unmapped = observation?.entries.filter(b => b.cycle === "bossWeekly" && b.variant_id === null).length ?? 0;
  const current = view.week.start_day === view.week.current_start && view.managed;
  async function check() {
    if (loading || !view.character) return;
    setLoading(true); setError(null);
    try { setResult(await checkBossApi(view.character.identity.ocid, view.week.start_day, view.selection_revision)); onRefresh?.(); }
    catch (e) {
      setError(bossError(e));
      if (typeof e === "object" && e !== null && "retry_after_seconds" in e && typeof e.retry_after_seconds === "number") setRetryAt(Date.now() + e.retry_after_seconds * 1000);
    } finally { setLoading(false); }
  }
  return <details><summary>API 완료 상태 (참고 정보)</summary>
    <p className="help-text">현재 선택한 관리 캐릭터만 명시적으로 조회합니다. 수동 기록은 자동 수정·취소하지 않습니다. API에는 클리어 날짜와 파티 인원이 없으므로 직접 확인해 주세요.</p>
    <button type="button" disabled={disabled || loading || !current} onClick={() => { if (Date.now() < retryAt) { setError("API 호출 대기 중입니다. 잠시 후 다시 눌러 주세요."); return; } void check(); }}>{loading ? "API 조회 중…" : "API 완료 상태 확인"}</button>
    {!current && <p>현재 주차의 관리 캐릭터만 조회·반영할 수 있습니다.</p>}
    {error && <p role="alert" className="notice error">{error}</p>}
    {observation && <><p>조회 기준일 {observation.query_date} · 관측 시각 {new Date(observation.observed_at * 1000).toLocaleString("ko-KR")}</p>
      {observation.query_date !== view.week.today && <p>지난 조회 결과입니다. 오늘 기준으로 다시 조회해야 반영할 수 있습니다.</p>}
      {observation.entries.length === 0 && <p>API 보스 정보가 없습니다. 기존 기록은 유지합니다.</p>}
      <p className="help-text">현재 수동 관리 난이도와 기존 기록에 해당하는 관측만 표시합니다. API로 관리 목록을 자동 변경하지 않습니다.</p>
      {unmapped > 0 && <p>매핑 미확인 주간 항목 {unmapped}건 · 반영 불가</p>}
      {visible.length === 0 && observation.entries.length > 0 && <p>현재 관리 목록에 해당하는 API 관측이 없습니다. 기존 목록과 기록을 유지합니다.</p>}
      {visible.map((b, i) => <ApiRow key={`${observation.id}:${i}`} boss={b} observation={observation} view={view} disabled={disabled || loading || !current || observation.query_date !== view.week.today} mutate={mutate} />)}
    </>}
  </details>;
}
function ApiRow({ boss, observation, view, disabled, mutate }: { boss: ObservedBoss; observation: BossObservation; view: BossView; disabled: boolean; mutate: Mutate }) {
  const [selected, setSelected] = useState(false), [party, setParty] = useState("1"), [date, setDate] = useState("");
  const variant = view.catalog.find(v => v.id === boss.variant_id);
  const record = variant && view.records.find(r => r.clear_group === (variant.shared_limit ? variant.boss_id : variant.id));
  const plan = variant && view.plans.find(p => p.boss_id === variant.boss_id && p.enabled);
  const price = variant ? effectivePrice(view, variant, date || null) : null;
  const estimate = previewIncome(price, party);
  const allowed = !disabled && boss.completed && variant && !record && plan;
  const valid = estimate !== null && date >= view.week.start_date && date <= observation.query_date && (!variant?.party_max || BigInt(party) <= BigInt(variant.party_max));
  return <form className="boss-editor" onSubmit={e => { e.preventDefault(); if (!allowed || !selected || !valid) return; void mutate(() => saveBossClear({ ocid: view.character!.identity.ocid, week: view.week.start_day, variant_id: variant.id, completed: true, party: Number(party), clear_date: date, reason: "API 완료 관측을 사용자 확인 후 반영", expected_revision: 0, selection_revision: view.selection_revision, request_id: crypto.randomUUID(), observation_id: observation.id })); }}>
    <p>{boss.content_name} · {boss.difficulty} · 주간 · 스케줄러 {boss.registered === true ? "등록" : boss.registered === false ? "미등록" : "등록 여부 미확인"} · API {boss.completed ? "완료" : "미완료"} · {record ? `기존 기록: ${record.completed ? "완료" : "해제"} · 기존 기록 유지 (정정은 기록 편집)` : "수동 기록 없음"}</p>
    {!variant ? <p>매핑 미확인 · 반영 불가</p> : boss.completed && !record && <>
      {!plan && <p>먼저 이 보스를 관리 목록에 저장해 주세요.</p>}
      <fieldset disabled={!allowed}><legend>반영 전 확인</legend>
        <label><input type="checkbox" checked={selected} onChange={e => setSelected(e.target.checked)} />이 API 완료 기록 반영 선택</label>
        <label>파티 인원<input type="number" min="1" max={variant.party_max ?? 2147483647} step="1" required value={party} onChange={e => setParty(e.target.value)} /></label>
        <label>실제 클리어 날짜<input type="date" required min={view.week.start_date} max={observation.query_date} value={date} onChange={e => setDate(e.target.value)} /></label>
        <p>반영 예상 개인 수익: {money(estimate)} · 적용 1인 가격: {money(price)}</p>
        {price === null && <p>내장 가격 또는 적용 시점이 미확인되어 반영할 수 없습니다.</p>}
        <button type="submit" disabled={!selected || !valid}>선택 기록 반영</button>
      </fieldset>
    </>}
  </form>;
}
