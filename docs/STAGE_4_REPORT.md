# 4단계 완료 기록 — 본캐 지정 및 계정·월드 필터

작업일: 2026-10-28. AGENTS, 아키텍처, 개발 계획, 3단계 보고서와 기존 목록·키 관리 구현을 먼저 확인했다. 외부 maplemate 스킬은 사용하거나 참고하지 않았다. 5단계 관리 캐릭터 선택은 구현하지 않았다.

## 본캐 지정과 필터

전체 API 목록을 계정·월드별로 나누어 본캐 선택 후보로 제공한다. 선택 입력은 OCID이며 Rust가 마지막 성공 목록에서 해당 캐릭터와 부모 account_id·world_name을 찾는다. 프런트엔드가 임의로 제출한 새 계정/월드나 닉네임으로 소속을 추정하지 않는다. 동명이인도 OCID로 구분한다.

현재 본캐는 정확히 한 개만 저장한다. 최초 미지정 상태는 안내하며 자동으로 첫 캐릭터를 지정하지 않는다. 관리 후보는 저장된 본캐의 account_id와 world_name이 모두 같은 캐릭터로 Rust에서 계산하고 본캐 자신을 포함한다. 조회 배열 순서는 식별에 사용하지 않는다.

서비스는 마지막 성공 API 목록과 세대 번호를 메모리에 유지한다. UI는 OCID, 목록 세대, 현재 본캐 식별자, 변경 확인 여부만 저장 요청으로 보낸다. 오래된 목록 요청은 snapshot_changed로 거부한다. 저장 직전 DB의 본캐가 UI가 알고 있던 본캐와 다르면 main_changed로 거부한다. 따라서 키 변경·목록 재조회·다른 앱 인스턴스의 설정 변경과 엇갈린 저장을 막는다.

계정 또는 월드가 달라지는 본캐 변경은 확인창을 표시하고 Rust도 확인 여부를 검사한다. 취소는 저장 명령을 호출하지 않는다. 확인된 변경은 singleton 설정 행만 갱신하며 보스·메소·아이템 기록을 삭제하는 SQL은 없다. 관리 선택 테이블이나 초기화 기능도 미리 만들지 않았다.

## SQLite 선택과 최소 저장 구조

고정 스택과 기존 아키텍처에 맞춰 SQLite를 이번에 도입했다. JSON은 지금의 한 설정에는 간단하지만 이후 SQLite 이전이 추가로 필요하고 비교 후 원자적 갱신을 직접 관리해야 한다. sqlx/ORM은 한 행 설정에 비해 구성과 의존성이 늘어 rusqlite를 선택했다.

- Rust 추가 직접 의존성: `rusqlite 0.41.0`, default-features=false, bundled 기능만 사용.
- 최소 Rust 1.89이며 현재 Rust 1.99.1에서 컴파일 성공했다.
- SQLite C 소스를 함께 빌드하여 배포 PC의 별도 SQLite DLL/설치에 의존하지 않는다. C++ 개발 도구는 기존 환경을 사용했다.
- lock의 신규 패키지는 rusqlite, libsqlite3-sys 0.39.1, fallible-iterator, fallible-streaming-iterator 4개다. 별도 npm 의존성은 없다.

저장 위치는 Tauri `app_local_data_dir()` 아래 `ledger.sqlite3`다. 실제 Windows 확인 경로는 `C:\Users\10203\AppData\Local\com.mapleledger.app\ledger.sqlite3`다. 실행 파일·OneDrive 리포지토리와 분리했다.

테이블은 `main_character` 하나이며 singleton=1 기본키/체크 제약으로 최대 한 행만 저장한다. 필드는 account_id, ocid, world_name과 표시용 character_name/class/level뿐이다. API 키·전체 API 응답·관리 선택·장부는 넣지 않았다. API 키는 기존 Windows Credential Manager를 유지한다.

`migrations/001_main_character.sql`과 PRAGMA user_version=1을 같은 트랜잭션에서 적용한다. 별도 마이그레이션 프레임워크·테이블은 추가하지 않았다. 더 높은 DB 버전은 수정 없이 거부한다. 저장은 파라미터화 SQL과 Immediate 트랜잭션을 쓰고, 기존 본캐 비교·확인 검사·단일 행 갱신이 원자적으로 처리된다. DB I/O는 blocking worker에서 실행한다. 저장 실패는 안전한 코드/메시지로 안내하고 이전 선택을 유지한다. 설정 DB가 열리지 않아도 앱 전체나 API 키 설정을 강제 종료하지 않는다.

근거: [rusqlite 공식 문서](https://docs.rs/rusqlite/0.41.0/rusqlite/), [bundled 기능](https://docs.rs/crate/rusqlite/0.41.0/features), [Tauri 앱 로컬 경로 API](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html).

## API 데이터 변경 시 동작

- 아직 목록이 없으면 저장된 본캐를 복원하고 재조회 안내만 표시한다. 계정/월드 후보를 임의로 생성하지 않는다.
- 저장된 OCID가 목록에서 사라지거나 같은 닉네임의 다른 OCID로 바뀌면 missing으로 안내하고 설정을 보존한다.
- 같은 OCID의 account_id 또는 world_name이 저장 정보와 다르면 identity_changed로 안내하고 후보를 비운다. 자동 재연결하지 않는다.
- 식별 정보가 같고 이름·직업·레벨만 바뀌면 현재 조회의 표시 정보를 사용한다. 재실행 직후에는 마지막 저장 시 표시 정보가 나타나며 조회 후 최신 값을 표시한다.
- 정상 빈 API 목록도 안전하게 처리한다. 저장된 본캐를 즉시 삭제하지 않는다.
- 키 교체·삭제 또는 인증 실패 시 서비스 목록을 무효화하지만 DB의 본캐 설정은 지우지 않는다.

## UI

기존 목록 조회와 설정 화면을 유지했다. 본캐 영역에 현재 본캐·월드·직업·레벨, 미지정/조회 없음/식별 불일치 안내를 추가했다. 전체 목록은 접이식 본캐 선택 영역으로 표시하며 계정·월드 구분과 각 행의 본캐 지정 버튼을 제공한다. 선택된 행은 현재 본캐로 표시하고 추가 지정 버튼을 비활성화한다.

동일 계정·월드 후보 표는 별도로 닉네임·월드·직업·레벨 및 본캐 표시를 제공한다. 후보 표에는 아직 관리 선택 체크박스나 최대 6개 제한을 넣지 않았다. 다른 계정/월드 변경은 확인·취소 버튼과 Escape 취소를 제공한다. 설정 읽기/저장 오류, 처리 상태 및 성공 안내도 표시한다.

## 테스트와 실제 앱 검증

| 검증 | 결과 |
| --- | --- |
| npm run typecheck | 통과 |
| cargo fmt --check / git diff --check | 통과 |
| domain::tests 2개 | 같은 계정·월드 포함, 다른 계정/월드 제외, 본캐 포함, 동명이인 OCID 구분, 순서 변경, 빈 목록·본캐 없음·OCID 변경·계정 이동·월드 변경·이름 변경 검증 통과 |
| storage::tests 3개 | 저장/연결 재개, 변경 확인 거부·승인, 오래된 설정 비교 거부, 단일 행 유지, 무관한 테스트 이력 보존, 쓰기 실패 롤백, 미래 스키마 보존, 별도 프로세스 복원 통과 |
| services::tests 1개 | 기존 호출 제한/직렬화 관련 회귀 통과 |
| 실제 목록 조회 | 사용자 등록 키로 1회, 전체 후보 71개 표시 및 지정 버튼 활성화 확인 |
| 사용자 본캐 선택 | 사용자가 직접 지정, 저장 성공 및 본캐 포함 동일 계정·월드 후보 51개·4열 표 확인 |
| 실제 변경 취소 | 다른 계정/월드 후보의 확인창→취소, 취소 안내 및 DB 파일 해시 불변 확인 |
| 실제 앱 재실행 | 앱 완전 종료 후 기존 개발 바이너리를 다시 실행하여 본캐 복원·조회 데이터 없음 안내 확인. 추가 API 호출 없음 |
| 기존 API 키 설정 | 재실행 후 API 키 등록됨 확인. 사용자 키를 변경·삭제하지 않음 |

합성 테스트는 별도 임시 SQLite 파일과 합성 캐릭터만 사용했고 종료 시 정리했다. 사용자 DB를 실패/변경 승인 테스트에 사용하지 않았다. 실제 UI에서는 본캐 최초 지정과 다른 계정/월드 변경 취소를 확인했고, 변경 승인·저장 실패·누락/변경 데이터는 합성 단위 테스트로 검증했다. Windows UI의 모든 오류를 각각 강제로 재현한 것은 아니다.

이번 단계의 실제 Nexon API 호출은 1회다. 독립 실연동 테스트나 이미지 API를 다시 호출하지 않았다. 개발 TXT 키도 읽거나 수정하지 않았다. 본캐 변경·필터·설정 읽기·재시작 복원은 추가 API 요청을 하지 않았다. 개인 캐릭터 이름·OCID·계정 ID 원문은 테스트 로그나 보고서에 출력하지 않았다.

Windows 개발 앱은 단계 완료 시 1회 빌드(약 14.34초)했고 재실행은 컴파일된 실행 파일을 재사용했다. 이전 개발 워처는 편집 전에 종료하여 작은 수정마다 자동 재빌드하지 않게 했다. UI 자동화에서 접이식 영역의 비동기 노출·이미 펼쳐진 상태와 창 핸들 탐색 차이를 조정했으며, DB 및 앱의 실제 선택/복원 실패는 아니었다. 본캐 표시가 접근성 트리에서 닉네임과 합쳐진 경우에는 결합된 표시로 확인했다. 추가 API 요청이나 코드 재빌드는 발생하지 않았다.

최종 확인 후 테스트 앱·서버와 임시 로그는 정리했다. 사용자 본캐 설정과 Credential Manager 키는 그대로 유지했다. production/release 빌드·패키징·커밋·푸시는 수행하지 않았다. 변경하지 않은 API 파서와 키 저장소 전체 테스트는 이전 성공 결과를 참고했고 불필요하게 반복하지 않았다.

## 변경 파일

신규:
- `src-tauri/src/domain/mod.rs`
- `src-tauri/src/storage/mod.rs`, `src-tauri/src/storage/tests.rs`
- `src-tauri/migrations/001_main_character.sql`
- `src/features/characters/MainCharacterPanel.tsx`, `src/features/characters/CharacterTable.tsx`
- `src/shared/api/mainCharacter.ts`
- `docs/STAGE_4_REPORT.md`

수정:
- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`
- `src-tauri/src/services/mod.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`
- `src-tauri/build.rs`, `src-tauri/capabilities/default.json`
- `src/features/characters/CharacterListPanel.tsx`, `src/shared/api/characters.ts`
- `src/app/styles.css`
- `README.md`, `docs/ARCHITECTURE.md`, `docs/DEVELOPMENT_PLAN.md`

로컬 앱 데이터 DB는 Git 작업 영역 밖에 생성됐다. 기존 .gitignore의 SQLite/sidecar 규칙도 유지한다. API 명세서, 개발용 키 파일, API 클라이언트 및 설정 UI 자체는 수정하지 않았다. 기존 미추적 파일을 커밋하거나 삭제하지 않았다.

## 남은 과제 및 5단계 계획

- 전체 API 목록의 재시작 캐시는 아직 없다. 재실행 후 저장 본캐는 즉시 표시하지만 후보 목록은 수동 조회 후 표시한다.
- 본캐 한 개 설정을 먼저 구현했으므로 API 키별 다중 프로필, 관리 선택 및 장부 저장 구조는 아직 없다. 키 교체 시 본캐를 자동 삭제하거나 다른 캐릭터로 대체하지 않는다.
- 다음 단계는 본캐를 필수 포함하는 중복 없는 최대 6개 관리 선택이다. Rust 자격/상한 검증, 선택·해제 및 순서 저장, 재실행 복원, 본캐 변경 충돌 시 선택만 재설정하는 트랜잭션을 추가한다. 이력 데이터는 삭제하지 않는다.
- 이번 단계에서는 관리 선택 테이블·UI·관련 기능을 미리 만들지 않았다.

4단계 완료 후 멈추고 사용자 승인을 기다린다.
