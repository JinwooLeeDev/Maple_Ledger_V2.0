import { invoke } from "@tauri-apps/api/core";
import type { MainCharacter } from "./mainCharacter";

export interface BossVariant { id: string; boss_id: string; name: string; difficulty: string; solo_price: string | null; effective_date: string | null; source: string | null; party_max: number | null; shared_limit: boolean }
export interface BossPlan { boss_id: string; variant_id: string; enabled: boolean; revision: number }
export interface BossRecord {
  observation_id?: number | null;
  clear_group: string; variant_id: string; boss_id: string; name: string; difficulty: string; completed: boolean; party: number;
  clear_date: string | null; solo_price: string | null; personal_income: string | null; source: string;
  price_version: string; calculation_version: string; character: MainCharacter; recorded_at: number; reason: string; revision: number;
}
export interface BossTotal { amount: string; unpriced: number }
export interface BossView {
  week: { start_day: number; start_date: string; end_date: string; today: string; current_start: number };
  observation: BossObservation | null;
  scheduler?: BossObservation | null;
  selection_revision: number; catalog: BossVariant[]; characters: { character: MainCharacter; managed: boolean }[];
  character: MainCharacter | null; managed: boolean; plans: BossPlan[]; records: BossRecord[]; history: BossRecord[];
  character_total: BossTotal; total: BossTotal;
}
export interface ClearInput {
  ocid: string; week: number; variant_id: string; completed: boolean; party: number; clear_date: string | null;
  reason: string; expected_revision: number; selection_revision: number; request_id: string; observation_id?: number | null;
}
export const getBossView = (ocid: string | null, week: number | null) => invoke<BossView>("get_boss_view", { ocid, week });
export const saveBossPlan = (input: { ocid: string; variant_id: string; enabled: boolean; expected_revision: number; selection_revision: number }) => invoke<void>("save_boss_plan", { input });
export const saveBossClear = (input: ClearInput) => invoke<BossRecord>("save_boss_clear", { input });
export function bossError(cause: unknown): string {
  const code = typeof cause === "object" && cause !== null && "code" in cause ? cause.code : null;
  const messages: Record<string, string> = {
    boss_api_conflict: "기존 기록 또는 반영 이력이 있습니다. 정정은 기록 편집에서 해 주세요.",
    boss_api_stale: "현재 주차·오늘 기준의 API 결과가 필요합니다. 관리 선택과 날짜를 확인해 주세요.",
    boss_api_price_required: "내장 가격 또는 적용 시점이 미확인되어 API 완료를 반영할 수 없습니다.",
    request_in_progress: "이미 API 요청을 처리 중입니다.",
    rate_limited: "API 호출 대기 중입니다. 잠시 후 직접 다시 조회해 주세요.",
    authentication: "API 키의 권한과 유효성을 확인해 주세요.",
    key_missing: "설정에서 본인의 API 키를 등록해 주세요.",
    timeout: "API 응답 시간이 초과되었습니다. 기존 기록은 유지됩니다.",
    server: "넥슨 API 서버 오류입니다. 기존 기록은 유지됩니다.",
    network: "네트워크 연결을 확인해 주세요. 기존 기록은 유지됩니다.",
    response_parse: "API 응답이 없거나 구조를 확인할 수 없습니다. 기존 기록은 유지됩니다.",
    request_rejected: "API가 요청을 거부했습니다. 키의 계정·조회 권한을 확인해 주세요.",
    data_pending: "조회 기준일의 API 데이터가 아직 없습니다. 기존 기록은 유지됩니다.",
    offline_check: "오프라인 검증 모드에서는 실제 API를 호출하지 않습니다.",
    boss_party_invalid: "파티 인원은 1 이상의 정수로 입력해 주세요. 확인된 보스별 최대 인원을 초과할 수 없습니다.",
    boss_price_invalid: "가격은 1 이상의 메소 정수로 입력해 주세요. 저장 가능한 정수 범위를 초과할 수 없습니다.",
    boss_date_invalid: "클리어 날짜는 선택한 주차 안의 오늘 또는 과거 날짜여야 합니다.",
    boss_week_invalid: "주차가 올바르지 않습니다. 현재 주차로 이동해 주세요.",
    boss_conflict: "다른 저장으로 기록이 변경되었습니다. 화면 새로고침 후 다시 확인해 주세요.",
    selection_changed: "관리 캐릭터 선택이 변경되었습니다. 화면을 새로고침해 주세요.",
    identity_changed: "저장된 캐릭터 식별 정보가 달라 기록을 변경하지 않았습니다.",
    boss_not_managed: "현재 관리 캐릭터만 새 보스를 설정할 수 있습니다. 과거 기록은 유지됩니다.",
    boss_plan_required: "먼저 관리할 보스를 선택해 저장해 주세요.",
    boss_amount_overflow: "수익 합계가 저장 가능한 정수 범위를 초과하여 저장하지 않았습니다.",
    settings_version_unsupported: "현재 앱에서 지원하지 않는 DB 버전입니다. 기존 데이터는 변경하지 않았습니다.",
  };
  return typeof code === "string" && messages[code] ? messages[code] : "보스 기록을 읽거나 저장하지 못했습니다. 입력을 유지했습니다. 다시 확인해 주세요.";
}
export function money(value: string | null): string { return value === null ? "미산정" : `${BigInt(value).toLocaleString("ko-KR")} 메소`; }
export function previewIncome(price: string | null, party: string): string | null {
  if (price === null || !/^[0-9]+$/.test(price) || !/^[0-9]+$/.test(party)) return null;
  const p = BigInt(price), n = BigInt(party);
  if (p <= 0n || p > 9223372036854775807n || n <= 0n || n > 2147483647n) return null;
  return (p / n).toString();
}
export function currentWeekDay(timestamp = Date.now()): number {
  const day = Math.floor((timestamp + 9 * 3600000) / 86400000);
  return day - ((day % 7 + 7) % 7);
}

export interface ObservedBoss { registered?: boolean | null; content_name: string; difficulty: string; cycle: string; completed: boolean; variant_id: string | null }
export interface BossObservation { id: number; query_date: string; observed_at: number; entries: ObservedBoss[] }
export const checkBossApi = (ocid: string, week: number, selectionRevision: number) => invoke<BossObservation>("check_boss_api", { ocid, week, selectionRevision });
export function effectivePrice(view: BossView, variant: BossVariant, clearDate: string | null): string | null {
  return variant.effective_date !== null && (clearDate || view.week.start_date) > variant.effective_date ? variant.solo_price : null;
}
