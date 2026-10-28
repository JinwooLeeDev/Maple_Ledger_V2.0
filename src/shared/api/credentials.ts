import { invoke } from "@tauri-apps/api/core";

export interface ApiKeyStatus {
  registered: boolean;
}

export const getApiKeyStatus = () => invoke<ApiKeyStatus>("api_key_status");
export const saveApiKey = (apiKey: string) =>
  invoke<ApiKeyStatus>("save_api_key", { apiKey });
export const deleteApiKey = () => invoke<ApiKeyStatus>("delete_api_key");

// Never render a raw IPC/platform error: it may contain untrusted data.
export function credentialErrorMessage(error: unknown): string {
  const code = typeof error === "object" && error !== null && "code" in error
    ? error.code : undefined;
  if (code === "invalid_key_input") {
    return "API 키를 입력해 주세요. 공백·제어 문자 없이 1,280자 이내로 입력할 수 있습니다.";
  }
  if (code === "credential_data_unreadable") {
    return "저장된 키를 읽을 수 없습니다. 삭제 후 다시 등록해 주세요.";
  }
  return "보안 저장소 작업을 완료하지 못했습니다. Windows 앱에서 잠시 후 다시 시도해 주세요.";
}
