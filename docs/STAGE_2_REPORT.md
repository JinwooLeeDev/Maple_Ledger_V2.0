# 2단계 완료 기록 — API 키 등록 및 안전한 저장

작업일: 2026-10-28. 기존 AGENTS, 아키텍처, 개발 계획, 1단계 기록을 먼저 확인했다. 외부 maplemate 스킬은 사용하거나 참고하지 않았다. 이번 범위는 API 키 설정·저장·등록 상태·삭제이며 실제 Nexon API 호출 및 캐릭터 조회는 구현하지 않았다.

## 저장 방식과 선정 근거

Windows Credential Manager의 일반 자격 증명에 저장한다. 대상 이름은 `com.mapleledger.app/nexon-open-api`로 고정하며, Local 지속성을 명시하여 현재 Windows 사용자와 PC에서 앱 재실행 및 로그인 세션을 넘어 유지하는 정책이다. 다른 사용자의 자격 증명이나 다른 앱 항목을 검색·삭제하지 않는다.

최신 keyring 4.2 문서는 저장소를 직접 선택하는 앱에 keyring-core와 해당 플랫폼 저장소를 직접 연결하도록 안내한다. 이를 따라 `keyring-core 1.0.1`과 `windows-native-keyring-store 1.1.0`을 도입했다. 기본 기능과 검색 기능은 끄고 Windows 어댑터만 사용한다. crates.io 메타데이터·공식 문서·다운로드된 실제 소스의 의존성 및 로그 동작을 확인했다. 현재 keyring 계열에서 제공되는 Windows 전용 구현이며 최소 Rust 버전은 각각 1.85와 1.88로 현재 Rust 1.99.1에서 사용 가능하다. Tauri 2.12.5와 실제 컴파일·IPC·OS 저장소 접근이 성공했다.

대안인 직접 Win32 호출은 unsafe FFI와 버퍼 정리 관리가 늘고, DPAPI 파일은 암호문 파일의 수명·권한·복구를 직접 관리해야 한다. 전체 keyring 래퍼나 다중 플랫폼 저장소를 도입할 필요는 없었다.

근거: [keyring 4.2 앱 연결 지침](https://docs.rs/keyring/4.2.0/keyring/), [Windows 저장소와 지속성](https://docs.rs/windows-native-keyring-store/1.1.0/windows_native_keyring_store/), [Windows 어댑터 Cargo 명세](https://docs.rs/crate/windows-native-keyring-store/1.1.0/source/Cargo.toml), [Microsoft 자격 증명 지속성](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw).

## 구현 기능과 경계

- 기존 메인 화면을 유지하고 설정 이동 및 메인 복귀 버튼을 추가했다.
- API 키 입력은 기본 숨김이며 표시/숨김을 전환할 수 있다. 이는 새로 입력한 키에만 적용된다.
- 등록 상태는 처리 중·등록됨·미등록·확인 필요로 구분한다. 상태 조회 실패를 미등록으로 단정하지 않는다. 새로고침도 제공한다.
- 저장 시 기존 키를 교체하고 입력 필드를 비운다. 저장 후에는 원문 대신 등록 여부만 표시한다. 성공 문구는 “저장 완료. API 키 유효성은 아직 확인하지 않았습니다.”이다.
- 삭제 성공·실패 및 저장 실패 안내를 제공한다. 미등록 항목 삭제는 Rust에서 성공으로 처리한다. OS 작업 실패는 고정된 안전한 메시지로 보여주며 키나 원본 오류를 노출하지 않는다.
- Rust는 빈 입력, 내부 공백·제어 문자 및 Windows 비밀 크기 제한을 검사한다. 앞뒤 공백은 제거한다. 이 검사는 넥슨 API 유효성 검증이 아니다. 저장소 암호문 blob 제한 2560바이트에 맞춰 UTF-16 1280단위로 제한한다.
- React는 localStorage/sessionStorage/파일에 키를 저장하지 않는다. 입력은 DOM 메모리에만 있고 저장 시도·삭제·화면 이탈 시 비워진다. 저장된 키를 UI로 복원하는 IPC 명령은 없다.
- 명령은 `api_key_status`, `save_api_key`, `delete_api_key` 3개이며 응답은 `{ registered: boolean }`뿐이다. Rust 전용 읽기는 Zeroizing 버퍼로 감싼 뒤 폐기한다. 저장 입력도 Rust에서 zeroize로 정리한다.
- OS 호출은 blocking worker에서 실행하고 Mutex로 같은 키의 동시 접근을 직렬화한다. main 로컬 창에만 해당 세 명령을 허용하고 원격 origin 권한은 추가하지 않았다. 권한은 build.rs의 앱 command manifest에서 생성한다.
- 등록 상태 자체를 OS 저장소에서 판별하므로 이번 단계에 SQLite나 일반 설정 파일은 필요하지 않았다. DB·마이그레이션은 이후 데이터 영속화 단계에서 추가한다.

## 추가 의존성

| 구분 | 추가 | 이유 |
| --- | --- | --- |
| npm | @tauri-apps/api 2.12.4 | 공식 타입화 invoke 사용; 자체 IPC 구현 대신 선택 |
| Rust | keyring-core 1.0.1 | 자격 증명 API 및 오류 타입 |
| Rust | windows-native-keyring-store 1.1.0, default-features=false | Windows Credential Manager 전용 저장소; 검색 제외 |
| Rust | zeroize 1.9.1 | Rust 비밀 버퍼 폐기 시 메모리 정리 |
| Rust | serde 1, derive | 등록 상태·안전한 오류 DTO 직렬화; 기존 전이 의존성을 직접 선언 |

Cargo.lock에 새로 추가된 패키지는 keyring-core, Windows 어댑터, zeroize 3개다. byteorder, log, windows-sys, windows-link 및 serde는 기존 의존성 그래프에서 재사용했다. npm은 공식 API 패키지 1개만 추가했다. 별도 UI·라우팅·DB·HTTP·테스트 라이브러리는 설치하지 않았다.

## 검증 결과

개발용 TXT 키 대신 합성 문자열만 사용했다. Rust 테스트는 고유한 `com.mapleledger.app/test/...` 항목을 생성하고 RAII 정리로 삭제한다. 별도 자식 프로세스 읽기를 통해 인메모리 mock이 아닌 OS 저장소 지속성을 확인한다. 오류 시 키가 assertion 출력에 포함되지 않도록 실제 값 대신 boolean 비교를 사용한다.

| 검증 | 결과 |
| --- | --- |
| npm run typecheck | 통과 |
| cargo fmt --manifest-path src-tauri/Cargo.toml -- --check | 통과 |
| cargo test --manifest-path src-tauri/Cargo.toml --lib security::tests -- --test-threads=1 | 3개 통과, 약 0.07초(컴파일 별도) |
| 입력 검사 테스트 | 임의 비어 있지 않은 값 허용, 공백·제어 문자·UTF-16 크기 초과 거부 |
| 안전한 오류 테스트 | 비밀 bytes나 문자열을 포함한 라이브러리 오류를 고정된 공개 오류로 변환; 원문 미포함 확인 |
| 실제 OS 저장소 테스트 | 미등록→저장→Rust 읽기 일치→잘못된 교체 거부 후 이전 값 보존→정상 교체→별도 프로세스 읽기→삭제→미등록→재삭제 성공 |
| npm run tauri dev | 변경 부분 증분 dev 컴파일 약 12.30초, 앱 실행 성공 |
| 실제 Windows UI | 메인/설정 이동, 초기 숨김, 표시/숨김, 잘못된 입력 실패, 저장 성공·등록됨, 저장 후 빈 입력 확인 |
| 앱 완전 종료·재실행 | 같은 개발 실행 파일로 재빌드 없이 재실행, 등록됨 유지 및 원문 미복원 확인 |
| 실제 UI 삭제 | 삭제 성공→미등록→상태 새로고침 후 미등록 확인 |
| Git/코드 점검 | 키 파일 제외 유지, 키 파일 추적 없음, 평문 저장·키 로그·개발 TXT 읽기 코드 없음, diff 공백 검사 통과 |

UI 검증에서는 기존 사용자 키를 덮어쓰지 않도록 최초 미등록 상태를 확인한 후 합성 키를 입력했다. 앱 재실행 확인 후 합성 키를 삭제했다. 임시 UI 자동화 스크립트와 개발 로그는 target 아래에만 생성했고 작업 종료 시 정리했다. 앱과 서버도 종료했다. PowerShell 5의 UTF-8 스크립트 해석과 aria-pressed 버튼의 TogglePattern 차이는 자동화 측에서 처리했으며 앱 수정·추가 빌드는 필요하지 않았다.

실제 OS 접근 권한 차단·Credential Manager 장애를 강제로 재현하지는 않았다. 이 경우의 원본 오류 제거는 단위 테스트로, 실제 실패 UI는 잘못된 입력으로 확인했다. Windows 로그아웃/재부팅, 다른 Windows 사용자 및 다른 PC 테스트도 수행하지 않았다. 세션 이후 보존 정책은 Local 지속성 설정에 근거하며 앱 프로세스 재시작은 실제 검증했다.

실제 넥슨 API 호출은 0회다. 실제 키의 유효성은 미검증이다. production 프런트엔드 빌드, release 빌드, 배포 패키징, 무관한 전체 테스트, Git 커밋 및 푸시는 수행하지 않았다.

## 변경 파일

신규:
- `src/features/settings/SettingsPage.tsx`
- `src/shared/api/credentials.ts`
- `src-tauri/src/security/mod.rs` (관련 Rust 테스트 포함)
- `src-tauri/src/commands/mod.rs`
- `docs/STAGE_2_REPORT.md`

수정:
- `src/app/App.tsx`, `src/app/styles.css`
- `src-tauri/src/lib.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`
- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`
- `package.json`, `package-lock.json`, `.gitignore`
- `AGENTS.md`, `README.md`, `docs/ARCHITECTURE.md`, `docs/DEVELOPMENT_PLAN.md`

생성 command 권한 파일은 `/src-tauri/permissions/autogenerated/`에 생기며 build.rs로 재생성 가능하므로 Git에서 제외했다. 기존 API 레퍼런스, 개발 키 파일, 1단계 보고서 및 초기 조사 문서는 수정하지 않았다. 기존 미추적 파일을 커밋하거나 삭제하지 않았다.

## 보안상 한계와 다음 단계

Credential Manager는 사용자 단위 OS 보호이며 같은 Windows 사용자 권한의 악성 코드까지 차단하지 않는다. Rust 버퍼 정리는 WebView·IPC·JavaScript에 생긴 모든 복사본의 완전 소거를 보장하지 않는다. 개발 도구의 IPC 추적, 프로세스 덤프, 키 입력 화면 캡처에 실제 키를 노출하지 않도록 해야 한다. 앱은 원문 로그를 남기지 않고 브라우저 영구 저장 기능도 사용하지 않는다.

`secrets/nexon-api-key.txt`는 존재와 Git 제외 상태만 확인했고 내용을 읽지 않았다. 자동 등록·수정·삭제·배포 리소스 추가는 하지 않았다. 테스트 후 사용자 저장소에는 합성 키를 남기지 않았다.

다음은 사용자 승인 후 3단계 캐릭터 목록 조회다. 공식 API 레퍼런스를 다시 확인하고 Rust 내부에서만 저장된 키를 읽어 API 클라이언트와 최소 실호출 검증을 추가한다. 응답의 계정 관계를 유지하고 안전한 오류·429·캐시 처리를 구현한다. 이번에는 해당 기능을 시작하지 않았으며 2단계 완료 후 멈춘다.
