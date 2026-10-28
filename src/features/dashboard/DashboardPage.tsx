import { useEffect, useState } from "react";
import { dashboardError, getDashboard, refreshDashboardCard, type DashboardCardData } from "../../shared/api/dashboard";
import CharacterCard from "./CharacterCard";
import { getBossView, money, currentWeekDay, type BossTotal } from "../../shared/api/bosses";

export function DashboardCards({ cards, loadingOcid = null, onBoss }: { cards: DashboardCardData[]; loadingOcid?: string | null; onBoss?: (ocid: string) => void }) {
  return <div className="dashboard-grid">{cards.map((card) => <CharacterCard key={`${card.character.identity.account_id}:${card.character.identity.ocid}`} card={card} loading={loadingOcid === card.character.identity.ocid} onBoss={onBoss} />)}</div>;
}

export default function DashboardPage({ onManage, onBoss }: { onManage: () => void; onBoss: (ocid: string | null) => void }) {
  const [cards, setCards] = useState<DashboardCardData[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingOcid, setLoadingOcid] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  const [total, setTotal] = useState<{ total: BossTotal; label: string } | null>(null);
  const [totalError, setTotalError] = useState(false);
  useEffect(() => {
    let cancelled = false;
    let boundary = currentWeekDay();
    const load = () => { void getBossView(null, null).then(v => { if (!cancelled) { setTotal({ total: v.total, label: v.week.start_date }); setTotalError(false); } }).catch(() => { if (!cancelled) setTotalError(true); }); };
    const check = () => { if (currentWeekDay() !== boundary) { boundary = currentWeekDay(); load(); } };
    load(); const timer = window.setInterval(check, 30000); window.addEventListener("focus", check);
    return () => { cancelled = true; window.clearInterval(timer); window.removeEventListener("focus", check); };
  }, [refresh]);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true); setError(null);
      try {
        const snapshot = await getDashboard();
        if (cancelled) return;
        setCards(snapshot.cards); // SQLite and memory cache first, before network.
        for (const card of snapshot.cards) {
          if (cancelled) break;
          setLoadingOcid(card.character.identity.ocid);
          try {
            const updated = await refreshDashboardCard(card.character.identity.ocid, snapshot.selection_revision, refresh > 0);
            if (!cancelled) setCards((current) => current.map((value) => value.character.identity.ocid === updated.character.identity.ocid ? updated : value));
            if (updated.error && ["authentication", "key_missing", "credential_store_unavailable", "rate_limited"].includes(updated.error.code)) {
              if (!cancelled) setError(dashboardError(updated.error));
              break; // Key/quota failures are shared; avoid repeating them for every card.
            }
          } catch (cause: unknown) { if (!cancelled) setError(dashboardError(cause)); }
        }
      } catch (cause: unknown) { if (!cancelled) setError(dashboardError(cause)); }
      finally { if (!cancelled) { setLoading(false); setLoadingOcid(null); } }
    }
    void load();
    return () => { cancelled = true; };
  }, [refresh]);

  return <section className="dashboard" aria-labelledby="dashboard-heading">
    <div className="dashboard-toolbar">
      <div><h2 id="dashboard-heading">내 캐릭터</h2><p className="help-text">본캐를 먼저, 저장한 순서대로 표시합니다.</p></div>
      <div className="toolbar-actions">
        <button type="button" onClick={onManage}>캐릭터 관리</button>
        <button type="button" onClick={() => onBoss(null)}>주간 보스</button>
        <button type="button" className="primary-button" disabled={loading} onClick={() => setRefresh((value) => value + 1)}>카드 새로고침</button>
      </div>
    </div>
    {error && <p className="notice error" role="alert">{error}</p>}
    {loading && <p className="dashboard-loading" role="status">{cards.length ? "관리 캐릭터 정보를 확인하고 있습니다." : "저장된 관리 설정을 읽고 있습니다."}</p>}
    {!loading && !error && cards.length === 0 && <div className="dashboard-empty"><h3>아직 관리 캐릭터가 없습니다.</h3><p>캐릭터 관리에서 본캐와 관리할 캐릭터를 지정해 주세요.</p><button type="button" onClick={onManage}>관리 캐릭터 설정하기</button></div>}
    <div className="boss-dashboard-summary">{totalError ? "주간 수익을 읽지 못했습니다. 다시 확인해 주세요." : total ? `${total.label} 주차 · 현재 관리 캐릭터 ${money(total.total.amount)} · 미산정 ${total.total.unpriced}건` : "주간 수익을 읽고 있습니다."}</div>
    <DashboardCards cards={cards} loadingOcid={loadingOcid} onBoss={onBoss} />
    <p className="dashboard-attribution">Data based on NEXON Open API</p>
  </section>;
}
