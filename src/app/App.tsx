import { useState } from "react";
import SettingsPage from "../features/settings/SettingsPage";
import CharacterListPanel from "../features/characters/CharacterListPanel";
import DashboardPage from "../features/dashboard/DashboardPage";
import BossPage from "../features/bosses/BossPage";

export default function App() {
  const [page, setPage] = useState<"main" | "settings" | "characters" | "bosses">("main");
  const [bossOcid, setBossOcid] = useState<string | null>(null);
  const openBoss = (ocid: string | null) => { setBossOcid(ocid); setPage("bosses"); };
  if (page === "settings") return <SettingsPage onBack={() => setPage("main")} />;

  return (
    <main className="workspace">
      <header className="app-header">
        <div><p className="eyebrow">나만의 메이플 기록</p><h1>Maple Ledger</h1></div>
        <nav className="toolbar-actions" aria-label="앱 화면">
          {page !== "main" && <button type="button" onClick={() => setPage("main")}>대시보드</button>}
        <button type="button" className="primary-button" onClick={() => setPage("settings")}>설정</button>
        </nav>
      </header>
      {page === "main" ? <DashboardPage onManage={() => setPage("characters")} onBoss={openBoss} /> : page === "bosses" ? <BossPage initialOcid={bossOcid} onBack={() => setPage("main")} /> : <CharacterListPanel />}
    </main>
  );
}
