import { invoke } from "@tauri-apps/api/core";

export interface Character {
  ocid: string;
  world_name: string;
  character_name: string | null;
  character_class: string | null;
  character_level: number | null;
}
export interface Account {
  account_id: string;
  character_list: Character[];
}
export interface CharacterSnapshot { accounts: Account[]; version: number }

export const loadCharacters = () => invoke<CharacterSnapshot>("load_characters");

export function characterError(error: unknown): { code: string; message: string; waitSeconds: number } {
  const value = typeof error === "object" && error !== null ? error as Record<string, unknown> : {};
  const code = typeof value.code === "string" ? value.code : "unknown";
  const messages: Record<string, string> = {
    key_missing: "API 키가 등록되어 있지 않습니다. 설정에서 API 키를 등록해 주세요.",
    authentication: "API 키가 유효하지 않거나 조회 권한이 없습니다. 설정에서 키를 확인해 주세요.",
    credential_store_unavailable: "Windows 보안 저장소에서 키를 읽지 못했습니다. 설정에서 등록 상태를 확인해 주세요.",
    network: "넥슨 API에 연결하지 못했습니다. 인터넷 연결을 확인해 주세요.",
    timeout: "조회 시간이 초과되었습니다. 잠시 후 다시 불러와 주세요.",
    rate_limited: "요청이 제한되었습니다. 안내된 대기 시간 후 다시 불러와 주세요.",
    server: "넥슨 API 서버가 응답하지 못했거나 점검 중입니다. 잠시 후 다시 시도해 주세요.",
    response_parse: "응답 데이터 형식이 예상과 다릅니다. 기존 목록은 유지하며 새 데이터는 적용하지 않았습니다.",
    data_pending: "넥슨에서 데이터를 준비 중입니다. 잠시 후 다시 불러와 주세요.",
    request_rejected: "넥슨 API가 요청을 처리하지 못했습니다. API 키의 게임·서비스 설정을 확인해 주세요.",
    request_in_progress: "이미 캐릭터 목록을 조회 중입니다. 잠시 기다려 주세요.",
    client_unavailable: "API 연결을 준비하지 못했습니다. 앱을 다시 실행해 주세요.",
  };
  return {
    code,
    message: messages[code] ?? "캐릭터 목록을 불러오지 못했습니다. 잠시 후 다시 시도해 주세요.",
    waitSeconds: typeof value.retry_after_seconds === "number" && Number.isFinite(value.retry_after_seconds)
      ? Math.min(86400, Math.max(1, value.retry_after_seconds)) : 0,
  };
}
