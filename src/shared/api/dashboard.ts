import { invoke } from "@tauri-apps/api/core";
import type { MainCharacter } from "./mainCharacter";
import { characterError } from "./characters";

export interface DashboardCardData {
  character: MainCharacter;
  is_main: boolean;
  image_src: string | null;
  updated_at: number | null;
  error: { code: string; retry_after_seconds: number | null } | null;
  image_error: boolean;
}
export interface DashboardSnapshot { cards: DashboardCardData[]; selection_revision: number }
export const getDashboard = () => invoke<DashboardSnapshot>("get_dashboard");

const pending = new Map<string, Promise<DashboardCardData>>();
export function refreshDashboardCard(ocid: string, revision: number, force: boolean) {
  const key = `${revision}:${ocid}`;
  const current = pending.get(key);
  if (current) return current;
  const request = invoke<DashboardCardData>("refresh_dashboard_card", { ocid, expectedSelectionRevision: revision, force });
  pending.set(key, request);
  void request.finally(() => { if (pending.get(key) === request) pending.delete(key); }).catch(() => {});
  return request;
}

export function dashboardError(error: unknown): string {
  const code = typeof error === "object" && error !== null && "code" in error ? error.code : null;
  if (code === "identity_changed") return "저장된 월드와 조회 정보가 다릅니다. 기존 선택을 유지했습니다. 캐릭터 관리에서 확인해 주세요.";
  if (code === "selection_changed" || code === "candidate_missing") return "관리 선택이 변경되었습니다. 대시보드를 새로고침해 주세요.";
  if (code === "settings_unavailable" || code === "settings_version_unsupported") return "저장된 관리 설정을 읽지 못했습니다. 기존 데이터는 변경하지 않았습니다.";
  if (code === "rate_limited") {
    const wait = characterError(error).waitSeconds;
    return `요청이 제한되었습니다.${wait ? ` 약 ${wait}초 후` : " 잠시 후"} 새로고침해 주세요. 기존 정보는 유지합니다.`;
  }
  return characterError(error).message;
}
