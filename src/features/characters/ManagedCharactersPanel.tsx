import { useEffect, useRef, useState } from "react";
import { mainCharacterError, saveManagedCharacters, type MainCharacter, type MainView } from "../../shared/api/mainCharacter";

interface Props {
  view: MainView;
  snapshotVersion: number | null;
  disabled: boolean;
  onSaved: (view: MainView) => void;
  onBusyChange: (busy: boolean) => void;
}

export default function ManagedCharactersPanel({ view, snapshotVersion, disabled, onSaved, onBusyChange }: Props) {
  const savedIds = view.managed_characters.map((entry) => entry.character.identity.ocid);
  const [draft, setDraft] = useState(savedIds);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const pending = useRef(false);
  const mounted = useRef(false);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => { setDraft(savedIds); setError(null); }, [view.selection_revision, snapshotVersion]);

  const main = view.main;
  if (!main) return null;
  const canAdd = snapshotVersion !== null && view.status === "matched";
  const rows = new Map<string, { character: MainCharacter; status: string }>();
  for (const entry of view.managed_characters) {
    rows.set(entry.character.identity.ocid, { character: entry.character, status: snapshotVersion === null ? "no_data" : entry.status });
  }
  if (canAdd) for (const character of view.candidates) {
    rows.set(character.ocid, { character: { identity: { account_id: main.identity.account_id, ocid: character.ocid, world_name: character.world_name }, character_name: character.character_name, character_class: character.character_class, character_level: character.character_level }, status: "matched" });
  }
  const dirty = draft.length !== savedIds.length || draft.some((id, index) => id !== savedIds[index]);

  function toggle(ocid: string) {
    if (disabled || saving || ocid === main?.identity.ocid) return;
    setNotice(null);
    if (draft.includes(ocid)) { setDraft(draft.filter((id) => id !== ocid)); return; }
    if (draft.length >= 6) { setError("최대 6개까지 선택할 수 있습니다. 추가 캐릭터를 먼저 해제해 주세요."); return; }
    const existing = savedIds.includes(ocid);
    if (!canAdd && !existing) return;
    setError(null);
    setDraft([...draft, ocid]);
  }

  async function save() {
    if (pending.current || disabled || !main) return;
    pending.current = true;
    setSaving(true); onBusyChange(true); setError(null); setNotice(null);
    try {
      const result = await saveManagedCharacters(draft, snapshotVersion, main.identity, view.selection_revision);
      if (!mounted.current) return;
      onSaved(result);
      setNotice("관리 캐릭터 선택을 저장했습니다.");
    } catch (cause: unknown) { if (mounted.current) setError(mainCharacterError(cause)); }
    finally {
      pending.current = false;
      if (mounted.current) setSaving(false);
      onBusyChange(false);
    }
  }

  return <section className="managed-panel" aria-labelledby="managed-heading" aria-busy={saving}>
    <h3 id="managed-heading">관리 캐릭터 선택</h3>
    <p role="status">선택 {draft.length}/6 · 저장된 선택 {savedIds.length}개</p>
    <p className="help-text">본캐는 고정 포함됩니다. 추가 선택한 순서로 저장하며, 해제해도 기존 보스·메소·아이템 기록은 삭제하지 않습니다.</p>
    {draft.length >= 6 && <p className="selection-limit" role="status">최대 6개를 선택했습니다. 다른 캐릭터를 선택하려면 추가 선택을 먼저 해제해 주세요.</p>}
    {!canAdd && <p>새 캐릭터 추가는 본캐와 목록을 확인한 뒤 가능합니다. 저장된 선택은 유지하며 추가 캐릭터 해제는 가능합니다.</p>}
    {error && <p className="notice error" role="alert">{error}</p>}
    {notice && <p className="notice success" role="status">{notice}</p>}
    <div className="world-section"><table>
      <caption className="visually-hidden">관리 캐릭터 선택 목록</caption>
      <thead><tr><th scope="col">선택</th><th scope="col">순서</th><th scope="col">캐릭터</th><th scope="col">월드</th><th scope="col">직업</th><th scope="col">레벨</th><th scope="col">조회 상태</th></tr></thead>
      <tbody>{[...rows].map(([ocid, row], index) => {
        const selected = draft.includes(ocid);
        const fixed = ocid === main.identity.ocid;
        const blocked = !selected && (draft.length >= 6 || (!canAdd && !savedIds.includes(ocid)));
        return <tr key={ocid} className={selected ? "managed-selected" : undefined}>
          <td><input type="checkbox" aria-label={fixed ? "본캐 고정 선택" : `관리 후보 ${index + 1} 선택`} checked={selected} disabled={fixed || disabled || saving || blocked} onChange={() => toggle(ocid)} /></td>
          <td>{selected ? draft.indexOf(ocid) + 1 : "—"}</td>
          <td>{row.character.character_name?.trim() || "이름 정보 없음"}{fixed && <span className="main-badge">본캐 · 고정</span>}</td>
          <td>{row.character.identity.world_name}</td><td>{row.character.character_class || "정보 없음"}</td><td>{row.character.character_level === null ? "정보 없음" : `Lv. ${row.character.character_level}`}</td>
          <td>{row.status === "matched" ? "조회됨" : row.status === "no_data" ? "아직 조회하지 않음" : row.status === "missing" ? "현재 조회되지 않음" : "식별 정보 불일치 · 기존 선택 보존"}</td>
        </tr>;
      })}</tbody>
    </table></div>
    <div className="actions">
      <button type="button" className="primary-button" disabled={disabled || saving || !dirty} onClick={() => { void save(); }}>관리 선택 저장</button>
      <button type="button" disabled={disabled || saving || !dirty} onClick={() => { setDraft(savedIds); setError(null); setNotice("저장 전 변경을 취소했습니다."); }}>저장 전 변경 취소</button>
    </div>
  </section>;
}
