import { invoke } from "@tauri-apps/api/core";
import type { Character } from "./characters";

export interface MainIdentity { account_id: string; ocid: string; world_name: string }
export interface MainCharacter {
  identity: MainIdentity;
  character_name: string | null;
  character_class: string | null;
  character_level: number | null;
}
export interface MainView {
  main: MainCharacter | null;
  status: "unselected" | "no_data" | "matched" | "missing" | "identity_changed";
  candidates: Character[];
  snapshot_version: number | null;
  selection_revision: number;
  managed_characters: ManagedEntry[];
}
export interface ManagedEntry {
  character: MainCharacter;
  is_main: boolean;
  status: "matched" | "no_data" | "missing" | "identity_changed";
}
export const getMainCharacter = () => invoke<MainView>("get_main_character");
export const setMainCharacter = (ocid: string, snapshotVersion: number, expectedMain: MainIdentity | null, confirmed: boolean, expectedSelectionRevision: number) =>
  invoke<MainView>("set_main_character", { ocid, snapshotVersion, expectedMain, confirmed, expectedSelectionRevision });
export const saveManagedCharacters = (ocids: string[], snapshotVersion: number | null, expectedMain: MainIdentity, expectedSelectionRevision: number) =>
  invoke<MainView>("save_managed_characters", { ocids, snapshotVersion, expectedMain, expectedSelectionRevision });

export function mainCharacterError(error: unknown): string {
  const code = typeof error === "object" && error !== null && "code" in error ? error.code : undefined;
  switch (code) {
    case "confirmation_required": return "계정 또는 월드가 달라지는 변경은 확인이 필요합니다. 본캐 설정을 다시 읽고 선택해 주세요.";
    case "main_changed": return "다른 작업에서 본캐 설정이 변경되었습니다. 본캐 설정을 다시 읽고 선택해 주세요.";
    case "selection_changed": return "다른 작업에서 관리 선택이 변경되었습니다. 본캐 설정을 다시 읽고 선택해 주세요.";
    case "selection_limit": return "본캐를 포함하여 1개 이상, 최대 6개까지 선택할 수 있습니다.";
    case "main_required": return "본캐는 반드시 관리 캐릭터에 포함해야 하며 선택 해제할 수 없습니다.";
    case "selection_duplicate": return "같은 캐릭터를 중복 선택할 수 없습니다.";
    case "selection_scope": return "동일 계정·월드로 확인된 캐릭터만 새로 선택할 수 있습니다. 기존 미조회 선택은 유지하거나 해제할 수 있습니다.";
    case "snapshot_changed":
    case "candidate_missing":
    case "no_character_data": return "선택에 사용한 조회 정보가 달라졌습니다. 캐릭터 목록을 다시 불러온 뒤 선택해 주세요.";
    case "settings_version_unsupported": return "현재 앱보다 새로운 버전의 설정 DB입니다. 최신 앱으로 열어 주세요. 기존 설정은 유지했습니다.";
    default: return "본캐 설정을 읽거나 저장하지 못했습니다. 기존 설정은 유지했습니다. 잠시 후 다시 시도해 주세요.";
  }
}
