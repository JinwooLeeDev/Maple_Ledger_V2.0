import { useEffect, useRef, useState } from "react";
import type { Account, Character, CharacterSnapshot } from "../../shared/api/characters";
import { getMainCharacter, mainCharacterError, setMainCharacter, type MainView } from "../../shared/api/mainCharacter";
import CharacterTable from "./CharacterTable";
import ManagedCharactersPanel from "./ManagedCharactersPanel";

interface Selection { account: Account; character: Character }

export default function MainCharacterPanel({ snapshot }: { snapshot: CharacterSnapshot | null }) {
  const [view, setView] = useState<MainView | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [pending, setPending] = useState<Selection | null>(null);
  const [reload, setReload] = useState(0);
  const [managedBusy, setManagedBusy] = useState(false);
  const mounted = useRef(false);
  const cancelButton = useRef<HTMLButtonElement>(null);

  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => {
    let cancelled = false;
    setBusy(true);
    setPending(null);
    setError(null);
    getMainCharacter().then((result) => { if (!cancelled) setView(result); })
      .catch((cause: unknown) => { if (!cancelled) { setView(null); setError(mainCharacterError(cause)); } })
      .finally(() => { if (!cancelled) setBusy(false); });
    return () => { cancelled = true; };
  }, [snapshot?.version, reload]);
  useEffect(() => { if (pending) cancelButton.current?.focus(); }, [pending]);

  async function save(selection: Selection, confirmed: boolean) {
    if (!snapshot || !view || busy) return;
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const result = await setMainCharacter(selection.character.ocid, snapshot.version, view.main?.identity ?? null, confirmed, view.selection_revision);
      if (!mounted.current) return;
      setView(result);
      setPending(null);
      setNotice(`본캐와 관리 선택을 함께 저장했습니다. 관리 ${result.managed_characters.length}개이며, 이전 본캐는 자동으로 추가하지 않았습니다.`);
    } catch (cause: unknown) {
      if (mounted.current) { setError(mainCharacterError(cause)); setPending(null); }
    } finally { if (mounted.current) setBusy(false); }
  }

  function select(account: Account, character: Character) {
    if (busy || managedBusy || !view) return;
    const current = view.main?.identity;
    const selection = { account, character };
    if (current && (current.account_id !== account.account_id || current.world_name !== character.world_name)) {
      setNotice(null);
      setPending(selection);
    } else { void save(selection, false); }
  }

  const main = view?.main;
  const hasCurrentData = !!snapshot && view?.snapshot_version === snapshot.version;
  const matched = hasCurrentData && view?.status === "matched";
  const accountNumber = snapshot?.accounts.findIndex((account) => account.account_id === main?.identity.account_id);
  return (
    <section className="main-character-panel" aria-labelledby="main-character-heading" aria-busy={busy}>
      <h2 id="main-character-heading">본캐릭터</h2>
      {busy && <p role="status">본캐 설정을 처리하고 있습니다.</p>}
      {error && <p className="notice error" role="alert">{error}</p>}
      {notice && <p className="notice success" role="status">{notice}</p>}
      {view && !main && <p role="status">본캐가 지정되지 않았습니다. 전체 캐릭터 목록에서 본캐 1개를 지정해 주세요.</p>}
      {main && <div className="current-main">
        <h3>현재 본캐</h3>
        <p>{main.character_name?.trim() || "이름 정보 없음"} · {main.character_class?.trim() || "직업 정보 없음"} · {main.character_level === null ? "레벨 정보 없음" : `Lv. ${main.character_level}`} · {main.identity.world_name}</p>
        <p className="help-text">{accountNumber !== undefined && accountNumber >= 0 ? `조회 목록의 계정 ${accountNumber + 1}` : "계정 소속은 캐릭터 목록 조회 후 확인합니다."}</p>
      </div>}
      {!snapshot && <p>캐릭터 조회 데이터가 없습니다. 목록을 불러오면 본캐 선택과 계정·월드별 후보를 확인할 수 있습니다.</p>}
      {hasCurrentData && view?.status === "missing" && <p className="notice error" role="alert">저장된 본캐가 이번 목록에 없습니다. 설정은 보존했습니다. 다시 조회하거나 다른 본캐를 선택해 주세요.</p>}
      {hasCurrentData && view?.status === "identity_changed" && <p className="notice error" role="alert">본캐의 계정 또는 월드 정보가 저장된 식별 정보와 다릅니다. 자동으로 변경하지 않았습니다. 다시 조회하거나 본캐를 명시적으로 변경해 주세요.</p>}
      <button type="button" onClick={() => setReload((value) => value + 1)} disabled={busy || managedBusy || !!pending}>본캐 설정 다시 읽기</button>
      {pending && <div className="confirm-main" role="alertdialog" aria-labelledby="confirm-main-heading" aria-describedby="confirm-main-description" onKeyDown={(event) => { if (event.key === "Escape" && !busy) setPending(null); }}>
        <h3 id="confirm-main-heading">계정 또는 월드가 다른 본캐로 변경</h3>
        <p id="confirm-main-description">{pending.character.character_name || "선택한 캐릭터"} ({pending.character.world_name})로 변경할까요? 추가 관리 선택을 초기화하고 새 본캐만 남깁니다. 기존 보스·메소·아이템 기록은 삭제하지 않습니다.</p>
        <div className="actions">
          <button ref={cancelButton} type="button" onClick={() => { setPending(null); setNotice("본캐 변경을 취소했습니다. 기존 설정을 유지합니다."); }} disabled={busy}>변경 취소</button>
          <button type="button" className="primary-button" onClick={() => { void save(pending, true); }} disabled={busy}>확인 후 본캐 변경</button>
        </div>
      </div>}
      {snapshot && <details className="main-options" open={!main || !!pending}>
        <summary>전체 캐릭터에서 본캐 선택·변경</summary>
        {snapshot.accounts.reduce((sum, account) => sum + account.character_list.length, 0) === 0 && <p>본캐로 지정할 캐릭터가 없습니다.</p>}
        {snapshot.accounts.map((account, index) => {
          const worlds = new Map<string, Character[]>();
          for (const character of account.character_list) {
            const list = worlds.get(character.world_name) ?? [];
            list.push(character); worlds.set(character.world_name, list);
          }
          return <section className="account-section" key={account.account_id} aria-labelledby={`account-${index}`}>
            <h3 id={`account-${index}`}>계정 {index + 1} · 캐릭터 {account.character_list.length}개</h3>
            {account.character_list.length === 0 && <p>이 계정에는 조회된 캐릭터가 없습니다.</p>}
            {[...worlds].sort(([a], [b]) => a.localeCompare(b, "ko")).map(([world, characters]) => <section className="world-section" key={world}>
              <h4>{world} · {characters.length}개</h4>
              <CharacterTable characters={characters} caption={`계정 ${index + 1}, ${world} 본캐 후보`} disabled={busy || managedBusy || !!pending || !hasCurrentData}
                selectedOcid={main?.identity.account_id === account.account_id && main.identity.world_name === world ? main.identity.ocid : undefined}
                onSelect={(character) => select(account, character)} />
            </section>)}
          </section>;
        })}
      </details>}
      {matched && <section className="filtered-candidates" aria-labelledby="filtered-candidates-heading">
        <h3 id="filtered-candidates-heading">본캐와 동일 계정·월드의 캐릭터</h3>
        <p role="status">관리 대상 후보 {view.candidates.length}개 (본캐 포함)</p>
        <p className="help-text">아래 관리 선택 목록에서 선택할 수 있습니다. 미조회 상태의 기존 선택도 별도로 보존합니다.</p>
      </section>}
      {view && <ManagedCharactersPanel key={view.main?.identity.ocid ?? "unselected"} view={view} snapshotVersion={hasCurrentData ? snapshot.version : null} disabled={busy || !!pending} onSaved={setView} onBusyChange={setManagedBusy} />}
    </section>
  );
}
