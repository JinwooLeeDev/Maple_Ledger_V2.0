import { useEffect, useRef, useState, type FormEvent } from "react";
import {
  credentialErrorMessage,
  deleteApiKey,
  getApiKeyStatus,
  saveApiKey,
} from "../../shared/api/credentials";

interface Props { onBack: () => void }
type Notice = { kind: "success" | "error"; text: string } | null;

export default function SettingsPage({ onBack }: Props) {
  const input = useRef<HTMLInputElement>(null);
  const [registered, setRegistered] = useState<boolean | null>(null);
  const [showKey, setShowKey] = useState(false);
  const [busy, setBusy] = useState(true);
  const [notice, setNotice] = useState<Notice>(null);

  useEffect(() => {
    let cancelled = false;
    getApiKeyStatus()
      .then((status) => { if (!cancelled) setRegistered(status.registered); })
      .catch((error: unknown) => {
        if (!cancelled) setNotice({ kind: "error", text: credentialErrorMessage(error) });
      })
      .finally(() => { if (!cancelled) setBusy(false); });
    return () => { cancelled = true; };
  }, []);

  async function refreshStatus() {
    setBusy(true);
    setNotice(null);
    try {
      const status = await getApiKeyStatus();
      setRegistered(status.registered);
    } catch (error: unknown) {
      setRegistered(null);
      setNotice({ kind: "error", text: credentialErrorMessage(error) });
    } finally { setBusy(false); }
  }

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    const key = input.current?.value ?? "";
    // The input is never restored from storage, even on failure.
    if (input.current) input.current.value = "";
    setShowKey(false);
    setBusy(true);
    setNotice(null);
    try {
      const status = await saveApiKey(key);
      setRegistered(status.registered);
      setNotice({ kind: "success", text: "저장 완료. API 키 유효성은 아직 확인하지 않았습니다." });
    } catch (error: unknown) {
      setNotice({ kind: "error", text: credentialErrorMessage(error) });
    } finally { setBusy(false); }
  }

  async function remove() {
    if (busy) return;
    setBusy(true);
    setNotice(null);
    if (input.current) input.current.value = "";
    setShowKey(false);
    try {
      const status = await deleteApiKey();
      setRegistered(status.registered);
      setNotice({ kind: "success", text: "저장된 API 키를 삭제했습니다." });
    } catch (error: unknown) {
      setNotice({ kind: "error", text: credentialErrorMessage(error) });
    } finally { setBusy(false); }
  }

  return (
    <main className="settings">
      <button type="button" className="text-button" onClick={onBack} disabled={busy}>← 메인 화면</button>
      <h1>설정</h1>
      <section className="settings-panel" aria-labelledby="api-key-heading" aria-busy={busy}>
        <h2 id="api-key-heading">넥슨 Open API 키</h2>
        <p className="description">내 API 키를 등록하세요. 저장된 키 원문은 다시 표시하지 않습니다.</p>
        <p className="registration-status" role="status">
          {busy ? "처리 중…" : registered === true ? "API 키 등록됨" : registered === false ? "API 키 미등록" : "등록 상태 확인 필요"}
        </p>
        <form onSubmit={(event) => { void save(event); }} autoComplete="off">
          <label htmlFor="api-key">{registered ? "새 API 키 (저장하면 기존 키 교체)" : "API 키"}</label>
          <div className="key-input-row">
            <input
              ref={input}
              id="api-key"
              type={showKey ? "text" : "password"}
              autoComplete="off"
              autoCapitalize="none"
              spellCheck={false}
              maxLength={1280}
              required
              disabled={busy}
              aria-describedby="key-help"
            />
            <button type="button" onClick={() => setShowKey(!showKey)} aria-pressed={showKey} disabled={busy}>
              {showKey ? "키 숨기기" : "키 표시"}
            </button>
          </div>
          <p id="key-help" className="help-text">이 Windows 사용자 계정의 보안 저장소에 저장합니다. API 유효성 검증은 수행하지 않습니다.</p>
          <div className="actions">
            <button type="submit" className="primary-button" disabled={busy}>API 키 저장</button>
            <button type="button" onClick={() => { void remove(); }} disabled={busy || registered === false}>저장된 API 키 삭제</button>
            <button type="button" onClick={() => { void refreshStatus(); }} disabled={busy}>등록 상태 새로고침</button>
          </div>
        </form>
        {notice && <p className={`notice ${notice.kind}`} role={notice.kind === "error" ? "alert" : "status"}>{notice.text}</p>}
      </section>
    </main>
  );
}
