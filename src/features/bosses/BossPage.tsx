import { BossApiPanel } from "./BossSupplements";
import { useEffect, useRef, useState } from "react";
import { bossError, effectivePrice, currentWeekDay, getBossView, money, previewIncome, saveBossClear, saveBossPlan, type BossPlan, type BossRecord, type BossVariant, type BossView, type ClearInput } from "../../shared/api/bosses";

export function BossTotals({ view }: { view: BossView }) {
  return <div className="boss-totals" aria-live="polite"><p>실제 기록된 캐릭터 주간 수익 <strong>{money(view.character_total.amount)}</strong><span>미산정 {view.character_total.unpriced}건</span></p><p>현재 관리 캐릭터 전체 <strong>{money(view.total.amount)}</strong><span>미산정 {view.total.unpriced}건</span></p></div>;
}

function PlanRow({ variants, plan, disabled, onSave }: { variants: BossVariant[]; plan?: BossPlan; disabled: boolean; onSave: (variant: string, enabled: boolean, revision: number) => void }) {
  const [variant, setVariant] = useState(plan?.variant_id ?? variants[0].id);
  const [enabled, setEnabled] = useState(plan?.enabled ?? false);
  return <div className="boss-plan-row"><label><input type="checkbox" checked={enabled} disabled={disabled} onChange={e => setEnabled(e.target.checked)} />{variants[0].name} 관리</label><select aria-label={`${variants[0].name} 관리 난이도`} disabled={disabled} value={variant} onChange={e => setVariant(e.target.value)}>{variants.map(v => <option key={v.id} value={v.id}>{v.difficulty}{v.solo_price === null ? " · 가격 미확인" : ""}</option>)}</select><button type="button" disabled={disabled || (enabled === (plan?.enabled ?? false) && variant === (plan?.variant_id ?? variants[0].id))} onClick={() => onSave(variant, enabled, plan?.revision ?? 0)}>설정 저장</button></div>;
}

export function BossEditor({ variants, record, plan, view, disabled, onSave }: { variants: BossVariant[]; record?: BossRecord; plan?: BossPlan; view: BossView; disabled: boolean; onSave: (input: ClearInput) => void }) {
  const [variantId, setVariantId] = useState(variants.some(v => v.id === record?.variant_id) ? record!.variant_id : plan?.variant_id ?? variants[0].id);
  const [completed, setCompleted] = useState(record?.completed ?? false);
  const [party, setParty] = useState(String(record?.party ?? 1));
  const [clearDate, setClearDate] = useState(record?.clear_date ?? (view.week.start_day === view.week.current_start ? view.week.today : ""));
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const variant = variants.find(v => v.id === variantId) ?? variants[0];
  const retain = record && record.variant_id === variantId && record.clear_date === (clearDate || null);
  const official = effectivePrice(view, variant, clearDate || null);
  const price = retain ? record.solo_price : official;
  const estimate = previewIncome(price, party);
  function submit(e: React.FormEvent) {
    e.preventDefault(); setError(null);
    if (!/^[0-9]+$/.test(party) || BigInt(party) < 1n || BigInt(party) > 2147483647n || (variant.party_max !== null && BigInt(party) > BigInt(variant.party_max))) { setError("파티 인원을 확인해 주세요."); return; }
    onSave({ ocid: view.character!.identity.ocid, week: view.week.start_day, variant_id: variantId, completed, party: Number(party), clear_date: clearDate || null, reason: reason.trim(), expected_revision: record?.revision ?? 0, selection_revision: view.selection_revision, request_id: crypto.randomUUID() });
  }
  return <form className={`boss-editor${record?.completed ? " boss-cleared" : ""}`} onSubmit={submit} aria-label={`${variants[0].name} 클리어 기록`}>
    <div className="boss-row-heading"><h3>{variants[0].name}</h3><span>{plan?.enabled ? "관리 중" : "관리 해제 · 기록 유지"}{record ? ` · revision ${record.revision}` : " · 미기록"}</span></div>
    <fieldset disabled={disabled}><legend className="visually-hidden">{variants[0].name} 기록 입력</legend>
      <label>난이도<select value={variantId} onChange={e => setVariantId(e.target.value)}>{variants.map(v => <option key={v.id} value={v.id}>{v.difficulty}</option>)}</select></label>
      <label className="boss-check"><input type="checkbox" checked={completed} onChange={e => setCompleted(e.target.checked)} />클리어</label>
      <label>파티 인원<input type="number" min="1" max={variant.party_max ?? 2147483647} step="1" required value={party} onChange={e => setParty(e.target.value)} /></label>
      <label>클리어 날짜<input type="date" min={view.week.start_date} max={view.week.today < view.week.end_date ? view.week.today : new Date((view.week.start_day + 6) * 86400000).toISOString().slice(0, 10)} value={clearDate} onChange={e => setClearDate(e.target.value)} /></label>
      <button type="submit" className="primary-button" disabled={!record && !completed}>기록 저장</button>
    </fieldset>
    <p className="boss-income">개인 수익 <strong>{money(estimate)}</strong>{!completed && " · 저장 시 클리어 상태만 반영합니다."}</p>
    <details><summary>가격·기록 근거</summary>
      <p className="help-text">적용 1인 가격: {money(price)} · {retain ? "기록 당시 가격 유지" : "공식 내장 자료"}{price === null && " — 가격 또는 적용 시점 미확인"}</p>
      <p className="help-text">내장 가격 출처: {variant.source ?? "공식 가격 미확인"} · 적용일: {variant.effective_date ?? "미확인"}</p>
      {record && <p className="help-text">저장값: {record.completed ? money(record.personal_income) : "클리어 해제 · 합계 제외"} · {record.price_version}<br />출처: {record.source}{record.observation_id && ` · 사용자 확인 API 관측 #${record.observation_id}`}</p>}
      <label>정정 사유 (선택)<input type="text" maxLength={300} value={reason} disabled={disabled} onChange={e => setReason(e.target.value)} /></label>
    </details>
    {variant.shared_limit && <p className="help-text">이 보스의 난이도는 주간 클리어 횟수를 공유합니다. 난이도 변경은 기존 기록의 정정으로 저장됩니다.</p>}
    {error && <p className="notice error" role="alert">{error}</p>}
  </form>;
}

export default function BossPage({ initialOcid, onBack }: { initialOcid: string | null; onBack: () => void }) {
  const [ocid, setOcid] = useState(initialOcid);
  const [week, setWeek] = useState<number | null>(null);
  const [view, setView] = useState<BossView | null>(null);
  const [refresh, setRefresh] = useState(0);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const locked = useRef(false);
  useEffect(() => {
    let cancelled = false; setLoading(true); setError(null);
    void getBossView(ocid, week).then(v => { if (!cancelled) setView(v); }).catch(e => { if (!cancelled) setError(bossError(e)); }).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [ocid, week, refresh]);
  useEffect(() => {
    const check = () => { if (view && currentWeekDay() !== view.week.current_start) setRefresh(n => n + 1); };
    const timer = window.setInterval(check, 30000); window.addEventListener("focus", check); document.addEventListener("visibilitychange", check);
    return () => { window.clearInterval(timer); window.removeEventListener("focus", check); document.removeEventListener("visibilitychange", check); };
  }, [view]);
  async function mutate(operation: () => Promise<unknown>) {
    if (locked.current) return; locked.current = true; setSaving(true); setError(null); setNotice(null);
    try { await operation(); setNotice("저장 완료"); setRefresh(n => n + 1); }
    catch (e) { setError(bossError(e)); }
    finally { locked.current = false; setSaving(false); }
  }
  const disabled = loading || saving;
  const groups = new Map<string, BossVariant[]>();
  for (const v of view?.catalog ?? []) groups.set(v.boss_id, [...(groups.get(v.boss_id) ?? []), v]);
  const active = new Set(view?.plans.filter(p => p.enabled).map(p => p.boss_id) ?? []);
  const scheduler = view?.scheduler;
  const registeredIds = new Set(scheduler?.entries.map(b => b.variant_id) ?? []);
  const past = new Set(view?.records.filter(r => scheduler ? !registeredIds.has(r.variant_id) : !active.has(r.boss_id)).map(r => r.boss_id) ?? []);
  return <section className="boss-page" aria-labelledby="boss-heading">
    <div className="dashboard-toolbar"><div><h2 id="boss-heading">주간 보스 관리</h2><p className="help-text">클리어 기록을 개인 메소 수익으로 환산합니다. 결정석은 모두 판매한다고 가정합니다.</p></div><button type="button" disabled={saving} onClick={onBack}>대시보드로</button></div>
    {error && <p className="notice error" role="alert">{error}</p>}{notice && <p className="notice success" role="status">{notice}</p>}
    {loading && <p role="status">저장된 보스 기록을 불러오는 중…</p>}
    <button type="button" disabled={disabled} onClick={() => { setNotice(null); setRefresh(n => n + 1); }}>기록 새로고침</button>
    {view && <>
      <div className="boss-navigation"><label>캐릭터<select aria-label="보스 관리 캐릭터" disabled={disabled} value={view.character?.identity.ocid ?? ""} onChange={e => { setNotice(null); setOcid(e.target.value); }}>{view.characters.map(c => <option key={c.character.identity.ocid} value={c.character.identity.ocid}>{c.character.character_name ?? "이름 정보 없음"} · {c.character.identity.world_name}{c.managed ? "" : " · 관리 해제"}</option>)}</select></label><div className="toolbar-actions"><button disabled={disabled || view.week.start_day < 7} onClick={() => { setNotice(null); setWeek(view.week.start_day - 7); }}>이전 주차</button><strong>{view.week.start_date} ~ {view.week.end_date} 직전</strong><button disabled={disabled || view.week.start_day >= view.week.current_start} onClick={() => { setNotice(null); setWeek(view.week.start_day + 7); }}>다음 주차</button><button disabled={disabled} onClick={() => setWeek(null)}>현재 주차</button></div></div>
      <p className="help-text">매주 목요일 00:00 KST 전환 · 과거 기록 보존{view.week.start_day !== view.week.current_start ? " · 과거 주차 조회/수정 중" : ""}</p>
      {view.character ? <p>{view.character.character_class ?? "직업 정보 없음"} · Lv. {view.character.character_level ?? "—"} · {view.character.identity.world_name}{!view.managed && " · 관리 해제 캐릭터: 기존 기록만 수정 가능"}</p> : <p>캐릭터 관리에서 본캐와 관리 캐릭터를 먼저 지정해 주세요.</p>}
      <BossTotals view={view} />
      <BossApiPanel key={`${view.character?.identity.ocid}:${view.week.start_day}`} view={view} disabled={disabled} mutate={mutate} onRefresh={() => setRefresh(n => n + 1)} />
      {view.character && <>
        <details className="boss-plans" open={active.size === 0}><summary>관리할 보스·난이도 선택 (캐릭터별 저장)</summary><p className="help-text">관리 선택을 해제해도 기록과 수익은 남습니다. 난이도 설정 변경도 기존 클리어를 변경하지 않습니다.</p>{[...groups.entries()].map(([id, variants]) => { const plan = view.plans.find(p => p.boss_id === id); return <PlanRow key={`${ocid}:${id}:${plan?.revision ?? 0}`} variants={variants} plan={plan} disabled={disabled || !view.managed} onSave={(variant, enabled, revision) => void mutate(() => saveBossPlan({ ocid: view.character!.identity.ocid, variant_id: variant, enabled, expected_revision: revision, selection_revision: view.selection_revision }))} />; })}</details>
        {!scheduler && active.size === 0 && <p className="notice">아직 관리할 보스가 없습니다. 위 목록에서 보스와 난이도를 선택하고 설정을 저장해 주세요.</p>}
        {scheduler && <SchedulerList view={view} disabled={disabled} onSave={input => void mutate(() => saveBossClear(input))} />}
        {!scheduler && <p className="help-text">데이터 출처: 수동 관리 · 정상 스케줄러 등록 목록이 없습니다.</p>}
        {!scheduler && [...groups.entries()].filter(([id]) => active.has(id)).map(([id, variants]) => { const record = view.records.find(r => r.boss_id === id), plan = view.plans.find(p => p.boss_id === id); return <BossEditor key={`${view.character!.identity.ocid}:${view.week.start_day}:${id}:${record?.revision ?? 0}:${plan?.revision ?? 0}`} variants={variants} record={record} plan={plan} view={view} disabled={disabled || (!view.managed && !record)} onSave={input => void mutate(() => saveBossClear(input))} />; })}
        {past.size > 0 && <details><summary>현재 관리 대상 외 기록 ({past.size})</summary><p className="help-text">관리 해제된 보스의 기록과 수익은 보존됩니다. 아래에서 기존 기록만 정정할 수 있습니다.</p>{[...groups.entries()].filter(([id]) => past.has(id)).map(([id, variants]) => { const record = view.records.find(r => r.boss_id === id), plan = view.plans.find(p => p.boss_id === id); return <BossEditor key={`${view.character!.identity.ocid}:${view.week.start_day}:${id}:${record?.revision ?? 0}:${plan?.revision ?? 0}`} variants={variants} record={record} plan={plan} view={view} disabled={disabled || (!view.managed && !record)} onSave={input => void mutate(() => saveBossClear(input))} />; })}</details>}
        <details className="boss-history"><summary>선택 주차 수정 이력 ({view.history.length})</summary>{view.history.length === 0 ? <p>기록이 없습니다.</p> : <ul>{view.history.map((r, i) => <li key={i}>{r.name} · {r.difficulty} · revision {r.revision} · {r.completed ? money(r.personal_income) : "클리어 해제"} · {r.party}인 · {new Date(r.recorded_at * 1000).toLocaleString("ko-KR")} {r.reason && `· ${r.reason}`}</li>)}</ul>}<p className="help-text">이력 전체를 합산하지 않습니다. 각 보스의 최신 완료 revision만 현재 합계에 포함됩니다.</p></details>
      </>}
    </>}
  </section>;
}


export function SchedulerList({ view, disabled, onSave }: { view: BossView; disabled: boolean; onSave: (input: ClearInput) => void }) {
  const snapshot = view.scheduler;
  if (!snapshot) return null;
  const rows = snapshot.entries.map(b => {
    const variant = view.catalog.find(v => v.id === b.variant_id);
    const record = view.records.find(r => r.variant_id === b.variant_id);
    const price = variant ? effectivePrice(view, variant, view.week.today < view.week.end_date ? view.week.today : null) : null;
    return { b, variant, record, price };
  });
  const known = rows.filter(r => r.price !== null);
  const expected = known.reduce((sum, r) => sum + BigInt(previewIncome(r.price, String(r.record?.party ?? 1))!), 0n).toString();
  const crystalTotal = known.reduce((sum, r) => sum + BigInt(r.price!), 0n).toString();
  return <section aria-label="스케줄러 등록 보스 목록">
    <h3>{view.character?.character_name} · 스케줄러 등록 보스 {rows.length}개</h3>
    <p>데이터 출처: 스케줄러 · 목록 조회일 {snapshot.query_date} · 표시 주차 {view.week.start_date} ~ {view.week.end_date} 직전</p>
    <p className="help-text">마지막 정상 등록 목록입니다. 실패·빈 등록 응답에서도 유지합니다. API 완료 여부는 조회일의 참고 정보이며 수동 기록을 변경하지 않습니다.</p>
    <p>등록 결정석 가격 합계: {money(crystalTotal)} · 가격 미확인 {rows.length - known.length}건</p>
    <p>예상 주간 개인 결정석 수익: {money(expected)} · 미산정 {rows.length - known.length}건</p>
    <p className="help-text">기록의 파티 인원을 사용하며 인원이 없으면 솔로(1인) 기준 추정치입니다. 미확인 가격은 합계에서 제외합니다. 실제 수익은 상단의 저장된 주간 기록 합계입니다.</p>
    {rows.map(({ b, variant, record, price }, i) => <article key={`${snapshot.id}:${i}`} className="boss-editor">
      <h3>{b.content_name} · {b.difficulty}</h3>
      <p>결정석 가격: {price === null ? "가격 미확인" : money(price)} · 기록된 클리어: {record?.completed ? "완료" : "미완료"} · 기록 파티 인원: {record ? `${record.party}인` : "미기록"} · 기록 개인 수익: {record?.completed ? money(record.personal_income) : "미발생"}</p>
      <p>API 완료 여부 ({snapshot.query_date}): {b.completed ? "완료" : "미완료"} · 클리어 시 예상 개인 수익: {money(previewIncome(price, String(record?.party ?? 1)))}</p>
      {variant ? <BossEditor key={`${view.week.start_day}:${variant.id}:${record?.revision ?? 0}`} variants={[variant]} record={view.records.find(r => r.clear_group === (variant.shared_limit ? variant.boss_id : variant.id))} plan={{ boss_id: variant.boss_id, variant_id: variant.id, enabled: true, revision: 0 }} view={view} disabled={disabled || !view.managed} onSave={onSave} /> : <p>내부 보스·난이도 매핑 미확인 · 가격 미확인 · 기록 입력 불가</p>}
    </article>)}
  </section>;
}
