# 3단계 완료 기록 — 실제 캐릭터 목록 조회

작업일: 2026-10-28. AGENTS, 아키텍처, 개발 계획, 2단계 보고서 및 API 레퍼런스의 서버·캐릭터 목록 요청·CharacterList 스키마를 먼저 확인했다. 외부 maplemate 스킬은 사용하거나 참고하지 않았다. 본캐 및 관리 캐릭터 선택은 구현하지 않았다.

## 실제 API 및 응답 검증

`GET https://open.api.nexon.com/maplestory/v1/character/list`에 `x-nxopen-api-key` 헤더만 전달했다. 쿼리 파라미터와 요청 본문은 없으며 기본 정보·이미지 등 추가 API는 호출하지 않았다.

에이전트가 실행한 실제 넥슨 요청은 총 2회다.

1. ignored Rust 테스트를 명시 실행하여 개발용 TXT 키로 독립 검증 1회. Windows 앱 실행 없이 HTTP 200과 응답 스키마를 검증했다.
2. 사용자가 앱 설정에서 직접 본인 키를 등록한 뒤 메인 화면 조회 버튼으로 1회. Windows Credential Manager→Rust Command→서비스→Nexon→React 표시 흐름을 확인했다.

두 경로 모두 성공했고 조회 집계는 **계정 3개, 캐릭터 71개**였다. 독립 검증 응답의 추가 필드·누락 필드·null 필드는 없었다. 원본 명세와 확인한 응답 사이에서 이번 목록의 필드 차이는 발견하지 못했다. 현재 시점·이 키의 응답에 대한 검증이며 모든 계정의 미래 응답까지 보장하지 않는다.

| 실제 확인된 필드 | 자료형/관계 |
| --- | --- |
| account_list | 루트의 계정 배열 |
| account_id | 계정 객체의 문자열 식별자 |
| character_list | 해당 부모 계정 소속 캐릭터 배열 |
| ocid | 캐릭터 문자열 식별자 |
| character_name | 닉네임 문자열 |
| world_name | 월드 문자열 |
| character_class | 직업 문자열 |
| character_level | 정수형 JSON number |

검증 출력에는 개수와 스키마 이름만 남겼고 키·계정 ID·OCID·캐릭터 이름·직업 값·응답 원문을 기록하지 않았다. 응답은 메모리에서만 처리했으며 임시 응답 파일이나 실제 개인 데이터 fixture를 만들지 않았다. 개발 키 파일은 Rust 테스트 코드에서만 읽었고 수정·삭제·자동 등록하지 않았다. 사용자 직접 등록 키는 그대로 보존했다.

## 구현 구조와 UI

`CharacterListPanel → load_characters IPC → CharacterSession → NexonClient` 흐름이다. Command가 기존 CredentialState를 통해 Rust에서만 키를 읽고 OS 저장소 접근 잠금을 유지한다. 키 원문은 React에 전달하지 않는다. reqwest Client는 서비스가 보관하여 연결을 재사용한다.

메인 화면의 기존 제목·설정 이동을 유지하면서 불러오기 버튼, 로딩, 등록 필요 안내, 안전한 오류, 빈 목록 안내를 추가했다. 성공 시 계정 순번→월드→닉네임·직업·레벨 표로 표시한다. 실제 Windows 접근성 트리에서 계정 그룹 3개, 각 그룹 수의 합 71개, 계정별 월드 표 10개를 확인했다. 전체 계정 ID와 OCID는 내부 DTO·React key로만 사용한다.

계정 ID는 부모 객체에 손실 없이 유지한다. 응답 배열을 계정 ID 및 월드·이름·OCID로 정렬하고 월드 그룹을 다시 구성하므로 API 반환 순서에 의존하지 않는다. 추가 필드는 역직렬화에서 무시한다. 닉네임·직업·레벨의 누락/null은 정보 없음으로 처리한다. 계정 ID·OCID·월드 및 배열 구조가 없거나 식별자가 비어 있거나 중복되면 전체 새 응답을 파싱 오류로 처리한다. 잘못된 소속을 추정하여 표시하지 않는다.

성공한 화면 스냅샷은 메모리에 유지한다. 네트워크·서버·파싱 실패 시 이전 조회임을 알리고 유지하며 인증·키 누락·저장소 실패 시 숨긴다. 설정으로 이동하면 목록이 해제되어 키 변경 뒤 이전 키의 데이터가 남지 않는다. 재실행 후 복원하는 영속 캐시는 구현하지 않았으며 DB도 만들지 않았다. 이는 이번 요청의 실제 목록 조회 범위에 맞춘 최소 구현이다.

## HTTP 클라이언트와 의존성

- `reqwest 0.13.5`: default-features=false, native-tls-no-alpn만 활성화. MSRV 1.85이며 Rust 1.99.1/Tauri 2.12.5에서 컴파일·실호출 성공.
- `serde_json 1.0.154`: 응답 JSON 파싱과 공식 오류명 해석. 기존 전이 의존성을 직접 선언했다.
- `tokio 1.54.1`, sync 기능: 비동기 요청 잠금. 기존 Tauri 런타임 의존성을 직접 선언했다.
- npm 추가 패키지는 없다.

Windows Schannel을 이용하여 별도 Rustls 암호화 빌드를 피했다. 동기 클라이언트는 blocking worker 관리가 추가되고, 직접 WinHTTP는 FFI 관리가 늘어 reqwest 비동기 클라이언트를 선택했다. 기본 charset·HTTP/2·쿠키·압축·업로드 기능은 켜지 않았다. lock에는 플랫폼 공통 18개 패키지가 추가됐으며 OpenSSL 등 다른 OS용 lock 항목이 Windows에서 빌드된다는 의미는 아니다. 실제 Windows 타깃은 native-tls/Schannel을 사용한다.

클라이언트는 HTTPS 전용, 고정 URL, 리다이렉트 차단, 민감 헤더 설정, 연결 5초·요청 전체 15초 제한, 응답 2MiB 상한을 적용한다. reqwest 자체 자동 재시도도 비활성화했다.

참고: [reqwest 문서](https://docs.rs/reqwest/0.13.5/reqwest/), [기능별 의존성](https://docs.rs/crate/reqwest/0.13.5/features).

## 오류와 호출 제한

원본 reqwest 오류·넥슨 오류 메시지·본문은 UI에 반환하지 않는다. 앱 오류 코드와 필요 시 대기 초만 반환하며 React는 코드별 고정 메시지만 표시한다.

| 상황 | 처리 |
| --- | --- |
| 키 미등록 | 요청 전에 key_missing, 설정 등록 안내 |
| 저장소 접근/읽기 실패 | credential_store_unavailable, 원문 미노출 |
| HTTP 401/403, 400 OPENAPI00005 | authentication, 키 또는 권한 확인 안내 |
| 연결/TLS/전송 실패 | network |
| 요청 시간 초과 | timeout |
| HTTP 429 또는 OPENAPI00007 | rate_limited, 대기 후 수동 재조회 |
| 5xx 및 공식 점검 오류 | server |
| OPENAPI00009 | data_pending |
| 그 외 거부/상태 | request_rejected |
| JSON/자료형/필수 식별 관계 오류 | response_parse |
| 정상 빈 배열 | 오류가 아닌 빈 목록 안내 |

프로젝트 레퍼런스는 ErrorMessage의 필드 구조를 제공하지만 세부 오류 코드 설명은 없으므로 [넥슨 공식 오류 표](https://openapi.nexon.com/ko/guide/request-api/)로 보완했다. 특히 유효하지 않은 키의 400/OPENAPI00005를 일반 입력 오류와 구분했다. 레퍼런스 원문은 수정하지 않았다.

요청 중 버튼 비활성화 및 프런트 pending 참조, 백엔드 try_lock으로 동시 중복 요청을 차단한다. 로컬 2초 간격은 클릭 억제 정책이며 넥슨 공식 쿼터값이 아니다. 429의 Retry-After가 정수 초이면 적용하고 없거나 HTTP-date 형식이면 60초를 대기한다. UI에 대기 초를 표시하고 자동 재시도는 하지 않는다. 키 저장·삭제와 목록 요청은 같은 서비스 잠금을 사용하여 변경 도중 요청이 엇갈리지 않게 한다.

## 검증 결과

| 검증 | 결과 |
| --- | --- |
| npm run typecheck | 통과 |
| cargo fmt / git diff --check | 통과 |
| nexon 일반 테스트 4개 | 필드·순서·소속 보존, 추가/누락/null·빈 목록·자료형·중복 식별자, 오류 안전성, 로컬 서버 헤더·파싱·타임아웃·전송 오류 통과 |
| 명시적 live_character_list_schema 1개 | HTTP 200, 3계정/71캐릭터, 추가/누락/null 필드 없음 |
| security 기존 테스트 3개 | 합성 키 저장·교체·삭제·별도 프로세스 복원 및 안전한 오류 회귀 통과 |
| services 테스트 1개 | 동시 잠금과 cooldown, HTTP 생성 전 재호출 차단, 키 변경 시 cooldown 초기화 통과 |
| 실제 Windows UI | 미등록 안내, 합성 키 저장·삭제 IPC, 사용자 키 직접 등록 후 실제 계정·월드별 목록 표시 확인 |

일반 테스트는 외부 API를 호출하지 않았다. 실제 빈 목록·인증 오류·429·서버 오류를 만들기 위해 넥슨에 추가 요청하지 않았으며 합성 JSON/상태 매핑으로 검증했다. 실제 UI에서는 미등록과 성공 경로를 확인했으며 모든 실패 상태를 Windows UI로 각각 재현한 것은 아니다.

최초 전송 테스트는 닫힌 Windows 포트가 연결 실패 대신 클라이언트 시간 초과로 판정되어 기대값이 실패했다. 원인을 확인한 뒤 로컬 서버의 비정상 HTTP 응답으로 non-timeout 경로를 결정적으로 확인하도록 수정했고 해당 테스트만 다시 실행하여 통과했다. 다른 성공 테스트는 필요 없이 반복하지 않았다. 자동화 도중 디버그 콘솔 핸들 및 React 요약 문구의 분할 텍스트 때문에 화면 탐색이 실패했지만 실제 앱 오류는 아니었다. 앱 창 핸들과 계정 그룹·표 개수로 확인했으며 추가 API 요청은 하지 않았다.

개발 앱의 증분 컴파일은 최초 약 14.15초였고, 서비스 테스트 추가 시 실행 중인 워처가 약 12.69초의 증분 재시작을 수행했다. 서비스 테스트는 외부 호출 없이 통과했고 재시작 후 실제 API를 다시 호출하지 않았다. 전체 release 빌드·패키징·무관한 전체 테스트·커밋·푸시는 하지 않았다.

## 변경 파일

신규:
- `src-tauri/src/nexon/mod.rs`
- `src-tauri/src/nexon/tests.rs`
- `src-tauri/src/services/mod.rs`
- `src/features/characters/CharacterListPanel.tsx`
- `src/shared/api/characters.ts`
- `docs/STAGE_3_REPORT.md`

수정:
- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`
- `src-tauri/src/security/mod.rs` (Rust 내부 읽기 가시성만 변경)
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`
- `src-tauri/build.rs`, `src-tauri/capabilities/default.json`
- `src/app/App.tsx`, `src/app/styles.css`
- `README.md`, `docs/ARCHITECTURE.md`, `docs/DEVELOPMENT_PLAN.md`

개발용 키와 API 레퍼런스는 유지했다. 키 저장 UI 자체와 앱 식별자도 변경하지 않았다. 개발 키는 Git 제외 상태이며 앱 패키지 리소스로 추가하지 않았다. 실제 응답 임시 파일은 생성하지 않았다.

## 남은 과제와 다음 단계

- 목록은 현재 화면 메모리 스냅샷만 유지한다. SQLite 스키마·마이그레이션·오프라인 재시작 캐시는 후속 저장 단계의 설계 대상이다.
- 정수 초가 아닌 Retry-After 날짜 형식은 보수적 기본 대기를 사용한다. 로그아웃·재부팅 및 다른 Windows 사용자 환경은 이번에 재검증하지 않았다.
- 키 저장 성공은 별도의 유효성 보증이 아니며, 목록 조회 성공은 해당 API의 그 시점 접근 성공을 의미한다.
- 사용자 키는 Credential Manager에 유지했고 개발 키 자동 등록은 하지 않았다. 등록 상태의 원문 조회 IPC도 여전히 없다.
- 다음 4단계는 본캐 1개 지정 및 동일 account_id·world_name 후보 필터링이다. 해당 기능, 최대 6개 관리 선택, 이미지·보스·장부는 아직 구현하지 않았다.

3단계 완료 후 사용자 승인을 기다린다. 사용자가 확인할 수 있도록 개발 앱은 열어 두며 창을 닫으면 개발 세션도 종료할 수 있다.
