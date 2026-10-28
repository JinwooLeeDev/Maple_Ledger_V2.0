import { useState } from "react";
import { dashboardError, type DashboardCardData } from "../../shared/api/dashboard";

export default function CharacterCard({ card, loading = false, onBoss }: { card: DashboardCardData; loading?: boolean; onBoss?: (ocid: string) => void }) {
  const [failedImage, setFailedImage] = useState<string | null>(null);
  const [loadedImage, setLoadedImage] = useState<string | null>(null);
  const hasImage = !!card.image_src && card.image_src !== failedImage;
  const character = card.character;
  const name = character.character_name?.trim() || "이름 정보 없음";
  return <article className={`character-card${card.is_main ? " character-card-main" : ""}`} aria-label={card.is_main ? "본캐 캐릭터 카드" : "관리 캐릭터 카드"} aria-busy={loading}>
    {card.is_main && <span className="card-main-label">본캐</span>}
    <div className="avatar-frame">
      <img src={hasImage ? card.image_src! : "/avatar-placeholder.svg"} alt={hasImage ? `${name} 캐릭터 이미지` : "대체 캐릭터 이미지"}
        width="104" height="104" draggable={false} onError={() => setFailedImage(card.image_src)} onLoad={() => { if (hasImage) setLoadedImage(card.image_src); }} />
    </div>
    <h3>{name}</h3>
    <p className="card-details">{character.character_class || "직업 정보 없음"} · {character.character_level === null ? "레벨 정보 없음" : `Lv. ${character.character_level}`}</p>
    <p className="card-world">{character.identity.world_name}</p>
    {loading ? <p className="card-state" role="status">정보 갱신 중…</p>
      : card.error ? <p className="card-error" role="status">{dashboardError(card.error)}</p>
      : card.image_error || (failedImage !== null && failedImage === card.image_src) ? <p className="card-state">이미지를 불러오지 못했습니다.{hasImage ? " 이전 이미지를 표시합니다." : " 대체 이미지를 표시합니다."}</p>
      : <p className="card-state">{card.updated_at ? `갱신 ${new Date(card.updated_at * 1000).toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit" })}` : "저장된 정보"}</p>}
    {hasImage && loadedImage === card.image_src && <span className="visually-hidden" role="status">캐릭터 이미지 표시 완료</span>}
    {onBoss && <button type="button" className="boss-card-button" onClick={() => onBoss(character.identity.ocid)}>보스 관리</button>}
  </article>;
}
