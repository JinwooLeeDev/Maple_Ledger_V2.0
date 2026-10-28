import { useEffect, useRef, useState } from "react";
import { characterError, loadCharacters, type CharacterSnapshot } from "../../shared/api/characters";
import MainCharacterPanel from "./MainCharacterPanel";

export default function CharacterListPanel() {
  const [snapshot, setSnapshot] = useState<CharacterSnapshot | null>(null);
  const [loadedAt, setLoadedAt] = useState<Date | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [blockedUntil, setBlockedUntil] = useState(0);
  const [now, setNow] = useState(Date.now());
  const mounted = useRef(false);
  const pending = useRef(false);

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  useEffect(() => {
    if (!blockedUntil) return;
    const timer = window.setInterval(() => {
      const time = Date.now();
      setNow(time);
      if (time >= blockedUntil) setBlockedUntil(0);
    }, 1000);
    return () => window.clearInterval(timer);
  }, [blockedUntil]);

  async function load() {
    if (pending.current || Date.now() < blockedUntil) return;
    pending.current = true;
    setLoading(true);
    setError(null);
    try {
      const result = await loadCharacters();
      if (!mounted.current) return;
      setSnapshot(result);
      setLoadedAt(new Date());
    } catch (cause: unknown) {
      if (!mounted.current) return;
      const failure = characterError(cause);
      if (["key_missing", "authentication", "credential_store_unavailable"].includes(failure.code)) {
        setSnapshot(null);
        setLoadedAt(null);
      }
      setError(failure.message);
      if (failure.waitSeconds) {
        setNow(Date.now());
        setBlockedUntil(Date.now() + failure.waitSeconds * 1000);
      }
    } finally {
      pending.current = false;
      if (mounted.current) setLoading(false);
    }
  }

  const characterCount = snapshot?.accounts.reduce((sum, account) => sum + account.character_list.length, 0) ?? 0;
  const wait = Math.max(0, Math.ceil((blockedUntil - now) / 1000));
  return (
    <section className="character-panel" aria-labelledby="characters-heading" aria-busy={loading}>
      <h2 id="characters-heading">내 캐릭터 목록</h2>
      <p className="description">등록한 API 키로 조회 가능한 캐릭터를 계정과 월드별로 표시합니다.</p>
      <button type="button" className="primary-button" onClick={() => { void load(); }} disabled={loading || wait > 0}>
        {loading ? "캐릭터 목록 불러오는 중…" : "캐릭터 목록 불러오기"}
      </button>
      {loading && <p role="status">넥슨에서 캐릭터 목록을 조회하고 있습니다.</p>}
      {wait > 0 && <p role="status">다시 조회하기까지 약 {wait}초</p>}
      {error && <p className="notice error" role="alert">{error}</p>}
      {!snapshot && !loading && !error && <p className="help-text">설정에서 API 키를 등록한 후 불러오기를 눌러 주세요.</p>}
      {snapshot && <>
        <p role="status">조회 결과: 계정 {snapshot.accounts.length}개 · 캐릭터 {characterCount}개</p>
        {loadedAt && <p className="help-text">{error ? "이전 조회 결과" : "마지막 조회"}: {loadedAt.toLocaleString("ko-KR")}</p>}
        {characterCount === 0 && <p>조회 가능한 캐릭터가 없습니다.</p>}
      </>}
      <MainCharacterPanel snapshot={snapshot} />
      <p className="help-text">Data based on NEXON Open API</p>
    </section>
  );
}
