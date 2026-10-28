# 7-C 완료 기록 — 결정석 가격 설정 및 API 완료 관측

작업일: 2026-10-28 KST. 사용자 요청한 7-C 범위만 구현했다. AGENTS, 아키텍처, 개발 계획, 보스 설계·가격 조사, 7-B 보고와 API 레퍼런스의 API 키 방식 scheduler 요청·응답·components를 확인했다. 기존 문서의 7-C/7-D 분할안보다 이번 사용자 요청을 우선했다. 기존 7-B를 재구현하지 않았으며 스택·의존성·CSS·카드 배치를 변경하지 않았다. maplemate를 사용하거나 참고하지 않았다.

## 1. 가격 관리

- 기존 카탈로그 54개와 공식 근거 가격 51개를 그대로 유지했다. 벨로나 하드·림보 익스트림·발드릭스 익스트림 3개는 null/공식 미확인으로 남겼다. 보조 자료 가격을 자동 적용하지 않는다.
- 보스·난이도별 수동 가격·확인 근거·저장 시각·revision을 SQLite에 저장한다. 모든 관리 캐릭터의 해당 난이도 신규 기록에서 사용하며 재실행 후 복원된다. 이는 과거 7-A의 기록별 입력만 지원하는 범위를 이번 요청에 따라 확장한 것이다.
- 내장 가격·출처·적용일과 수동 설정을 나란히 확인하고 저장·수정·해제할 수 있다. 해제 행도 revision을 유지해 오래된 화면의 저장 충돌을 검사한다.
- 우선순위: 명시적 기록별 가격 입력 → 동일 난이도·날짜의 기존 계산 스냅샷 → 현재 주차의 저장된 수동 설정 → 적용 시점이 확인되는 내장 가격 → 미산정. 기존 스냅샷 보존은 7-B 정책을 유지한다.
- 현재 수동 설정을 과거 주차에 자동 소급하지 않는다. 과거 정정은 기존 기록별 가격 입력을 사용한다. 신규 현재 주차 기록은 저장 당시 유효한 설정을 사용한다.
- 수동 설정의 가격 버전은 `setting:<variant_id>:<revision>`, 출처는 사용자 근거와 설정 저장 시각이다. 기록에는 실제 적용 가격·개인액·가격 버전·출처를 보존한다. 설정 변경/해제는 이미 저장된 기록을 갱신하지 않는다.
- 가격 자동 웹 갱신은 없다.

## 2. 파티 계산

기존 `party_floor_v1`을 재사용했다. Rust i64의 양의 가격을 필수 양의 정수 인원으로 나누며 소수점 이하는 버린다. 기본 인원은 1명이다. DTO 금액은 십진 문자열이고 UI 미리보기는 BigInt다.

- 1인/3인 계산, `4,040,000 / 3 = 1,346,666`, 합성 `10 / 3 = 3`, `30 / 3 = 10` 확인.
- 0·음수·소수·정수 범위 초과 가격/인원을 거부한다. 가격 i64 및 인원 i32 범위, 확인된 보스별 인원 제한과 합계 checked_add를 유지했다.
- 인원 정정 시 당시 가격으로 새 revision을 계산하고 기존 revision을 보존한다. 수정/취소/재완료 합계는 최신 revision만 포함한다.
- 검증되지 않은 게임 예외·추가 상한을 도입하지 않았다. 실제 게임의 소수점 처리·모든 특수 파티 예외를 새로 검증한 결과는 아니다.

## 3. 넥슨 API 연동

API 키 방식 `GET /maplestory/v1/scheduler/character-state`의 `ocid`, `boss_contents.content_name/difficulty/cycle/complete_flag`를 사용한다. Open ID 경로를 사용하지 않는다. 키는 기존 Windows Credential Manager에서 Rust만 읽고 기존 NexonClient의 민감 헤더·HTTPS·타임아웃·본문 상한·재시도 차단을 재사용한다. 키 원문 IPC와 개발 키 자동 등록은 추가하지 않았다.

- 보스 화면에서 사용자가 버튼을 누르면 현재 선택한 관리 캐릭터 한 명만 조회한다. 전체 관리 캐릭터 일괄 조회·렌더링·주차 변경·이미지 갱신에 의한 스케줄러 자동 호출은 없다.
- 백엔드는 관리 OCID·선택 revision·현재 주차를 조회 전/저장 시 검사한다. 기존 세션 try_lock과 로컬 2초 간격을 공유하고 429의 대기 시간을 보존한다. 자동 재시도하지 않는다.
- 실제 확인된 영문 난이도 `easy/normal/hard/chaos/extreme`를 내부 난이도에 연결한다. 주기는 `bossWeekly`만 허용한다. `듄켈`→`친위대장 듄켈`은 실제 응답에서 확인한 별칭이다. 나머지는 정확한 보스 이름을 사용한다.
- 일간/월간·알 수 없는 이름/난이도/주기·중복 항목은 임의 매핑하지 않는다. 관측 목록에서 매핑 미확인으로 표시하고 반영을 막는다. 카탈로그 54개 난이도의 합성 매핑을 검사했다. 모든 신규 난이도가 실제 완료된 응답으로 검증됐다는 뜻은 아니다.
- API 관측은 별도 테이블에 저장하며 수동 클리어/합계는 변경하지 않는다. 미완료·빈 목록·오류는 기존 기록을 취소하거나 삭제하지 않는다. 조회 실패 후 이전 관측은 기준일과 함께 유지한다.
- 오늘 기준 관측만 현재 주차에 수락한다. API가 제공하지 않는 실제 클리어 날짜와 파티 인원은 사용자 입력/확인이 필요하다. 날짜는 자동 채우지 않는다. 반영 체크와 예상 개인액을 확인하고 행별 반영 버튼을 누른다.
- 적용 가격이 없으면 API 반영을 막고 수동 가격 확인을 요구한다. 관리할 보스 설정도 먼저 저장해야 한다.
- 기존 수동 기록(해제 revision 포함)과 충돌하면 기존 기록을 유지한다. 정정은 기존 기록 편집에서 한다. API로 수동 기록을 직접 덮어쓰지 않는다.
- OCID+주차+clear group의 영구 수락 유일성과 기존 기록 유일성으로 새 관측/새 요청 ID를 반복 수락해도 추가 수익을 만들지 않는다. 동일 요청 재전송은 기존 결과를 반환하고 다른 요청의 중복 반영은 충돌 안내로 처리한다. 수동 취소 후 API 재조회로 되살리지 않는다.
- 클리어 snapshot의 observation_id와 수락 테이블로 API 관측을 수동 기록과 구분한다. 이후 정정도 최초 관측 출처를 보존한다.

## 4. 실제 HTTP 호출과 명세 차이

**실제 넥슨 HTTP 호출은 총 2회**, 독립 Python 검증에서만 실행했다. 개발 TXT 키를 메모리로 읽고 저장된 본캐 OCID는 사용자 DB를 읽기 전용으로 조회했다. 키·인증 헤더·OCID·캐릭터 이름·원본 응답을 파일/보고서에 남기지 않았다. 개발 키는 Credential Manager에 등록하지 않았다.

1. 명세의 `date=YYYY-MM-DD`를 포함한 요청: HTTP 400. 성공 응답 구조를 확인하지 못했다. 첫 요청의 오류 본문은 기록하지 않았으므로 400의 원인을 단정하지 않는다.
2. 명세에서 허용한 date 생략(오늘 조회): HTTP 200. 반환된 날짜는 `2026-10-28T00:00+09:00`이었다. `bossWeekly/bossDaily/bossMonthly`, 영문 난이도, 문자열 `true/false`, `듄켈` 명칭을 확인했다.

레퍼런스 date 설명은 YYYY-MM-DD지만 실제 성공 응답은 위 KST 시각 접미사를 포함했다. 클라이언트는 오늘 조회에서 date를 생략하고 확인된 `T00:00+09:00` 접미사 또는 날짜만 있는 응답을 엄격하게 정규화한다. 반환 기준일이 요청 시작 시점 및 저장 시점의 오늘과 다르면 반영하지 않는다. 임의의 시간대/날짜를 추정하지 않는다. API 레퍼런스 원문을 수정하지 않았다. 실제 응답 확인과 명세 기반 합성 테스트를 구분한다.

Windows UI 검증은 오프라인 모드였으므로 추가 넥슨 호출은 0회다. 실제 사용자 등록 키를 통한 UI 조회와 실제 완료 결과의 수락은 이번 검증에서 하지 않았다.

## 5. SQLite 및 데이터 보호

추가 마이그레이션 `004_boss_prices_observations.sql`로 v4를 지원한다. 기존 001~003은 변경하지 않았다.

| 추가 테이블 | 목적 |
| --- | --- |
| boss_price_settings | 보스·난이도별 수동 가격/해제·근거·revision·저장 시각 |
| boss_api_observations | 캐릭터·귀속 주차·기준일·관측 시각·선택 revision·당시 식별·관측 항목 |
| boss_api_acceptances | OCID+주차+clear group당 한 번 수락, 관측 및 기록 요청 연결 |

마이그레이션은 기존 Immediate 트랜잭션 안에서 실행한다. v3의 기존 기록·스냅샷·본캐·관리 선택 보존과 테이블 충돌 시 user_version=3 및 기존 데이터 롤백을 테스트했다. 기존 v1/v2 이전 검사도 통과했다. 수락 이력과 새 클리어 revision/현재 포인터는 같은 트랜잭션으로 저장하며, 수락 삽입 실패 시 클리어까지 롤백되는 것을 검사했다.

관리 선택 테이블의 자식으로 이력을 연결하지 않았다. 선택 변경/해제로 보스 기록·관측을 삭제하지 않는다. API 키 저장 방식은 그대로다. UI 검증은 OS 임시 디렉터리의 합성 DB만 사용했고 실제 사용자 DB의 검증 전후 SHA-256이 동일했다.

## 6. 변경 파일

신규:

- `src-tauri/migrations/004_boss_prices_observations.sql`
- `src-tauri/src/nexon/scheduler.rs`
- `src-tauri/src/storage/bosses/observations.rs`
- `src/features/bosses/BossSupplements.tsx`
- `scripts/check-bosses-native.mjs`
- `docs/STAGE_7C_REPORT.md`

수정:

- `src-tauri/src/domain/bosses.rs` — 관측 연결 필드, 기존 JSON 역호환
- `src-tauri/src/storage/mod.rs` — v4 이전
- `src-tauri/src/storage/bosses.rs` — 설정 가격 적용·관측 조회·원자적 수락
- `src-tauri/src/storage/bosses/tests.rs` — 가격·수락·이전·분리 UI fixture 테스트
- `src-tauri/src/storage/managed_tests.rs` — 정상 이전 버전 기대값 4
- `src-tauri/src/nexon/mod.rs` — scheduler 모듈 등록
- `src-tauri/src/services/mod.rs` — 기존 세션의 scheduler 요청·대기 제어 및 검사
- `src-tauri/src/commands/bosses.rs` — 가격 저장·명시적 API 확인 command
- `src-tauri/src/lib.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json` — command 등록과 좁은 권한
- `src/shared/api/bosses.ts` — DTO·IPC·오류·동일 가격 미리보기
- `src/features/bosses/BossPage.tsx` — 기존 화면에 기능 연결·적용 가격 및 관측 출처
- `scripts/check-bosses.mjs` — 수동 설정·API 확인/충돌 합성 UI 검증
- `docs/ARCHITECTURE.md`, `docs/DEVELOPMENT_PLAN.md`, `docs/BOSS_MANAGEMENT_DESIGN.md` — 현재 상태 및 새 정책 연결

기존 작업 트리는 다수 미추적 구현 파일을 포함한 상태였으며 사용자 변경을 되돌리지 않았다. 커밋·푸시·설치·배포 패키징은 하지 않았다.

## 7. 테스트 결과

| 실행 | 결과 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --offline` | 전체 40개 통과, 실호출/fixture용 3개 ignored |
| 추가 보강 `... --lib storage::bosses::tests::api_identity_mismatch_and_unknown_prices_require_confirmation --offline` | 1개 통과: 식별 불일치·미확인 가격 차단·수동 설정 후 수락·선택 revision 변경 |
| 보강 후 `... --lib services::tests --offline` | 1개 통과: scheduler도 대기 중 HTTP 생성 없이 차단 |
| `npm run typecheck` | 통과 |
| `node scripts/check-bosses.mjs` | 정수·미산정·KST·수동 가격 표시·API 입력·매핑 미확인·기존 해제 기록 충돌 통과 |
| `node scripts/check-dashboard.mjs` | 기존 카드 합성 검사 통과 |
| `... prepare_stage7c_native_ui_fixture -- --exact --ignored` | 명시적 임시 경로에서 합성 본캐/부캐·계획·관측 준비 성공 |
| `npm run tauri dev` | Windows 개발 앱 빌드/실행 성공, 개발 앱 빌드 1회 약 14.14초 |
| `node scripts/check-bosses-native.mjs` | 실제 WebView2 DOM 입력/버튼과 Tauri IPC를 통한 핵심 흐름 통과 |
| `node scripts/check-bosses-native.mjs --restore` | 실제 프로세스 재실행 후 설정·4개 revision·관측 출처·합계 복원 통과 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `git diff --check` | 통과 |

초기 컴파일에서 관측 필드 중복과 query_map 수명 오류를 수정했다. PowerShell 파이프의 한글 인코딩 손상을 UTF-8 지정 및 패치로 복구했고 실제 영문 API 값 매핑 테스트로 검증했다. 기존 마이그레이션 기대 버전과 SSR fixture의 clear group 기대값을 갱신했다. 네이티브 검사에서 주차 이동 후 접힌 details의 안내를 검사하던 스크립트를 펼친 후 검사하도록 수정했다. 앱 재실행 때 개발 서버가 함께 종료되어 Vite를 다시 실행한 뒤 동일 개발 실행 파일로 재검증했다. 실패 명령을 원인 확인 없이 반복하지 않았다.

## 8. Windows UI 결과

실제 Tauri/WebView2 개발 앱에서 합성 캐릭터 2개와 별도 SQLite를 사용했다. UI 결과는 JS 모의 invoke로 가장하지 않았으며, 가격과 기록 변경은 폼 입력/버튼으로 수행하고 저장 확인은 실제 read-only get_boss_view IPC로 확인했다. 스크린샷도 직접 확인했다. 기존 색상·폰트·카드 배치·CSS 변경은 없다.

- 내장 4,040,000과 사용자 수동 설정 10 표시·저장 확인.
- API 관측 선택, 3명 및 사용자가 입력한 날짜 확인 후 3메소 반영.
- 기존 기록/반복 반영 충돌 안내, 추가 수익 없음.
- 수동 설정 30으로 변경해도 저장된 3메소 유지.
- 클리어 취소 0 → 재완료 3메소, 수정 전 revision 보존.
- 기록별 명시적 가격 정정 30/3=10, revision 4와 최신액만 합산.
- 설정 해제 후 내장 가격 복귀 경로와 기존 10메소 유지.
- API 조회 버튼의 오프라인 오류 표시 후 기존 합계 유지.
- 알 수 없는 보스 매핑 미확인 표시, 과거 주차 API 조회/반영 제한 확인.
- 실제 프로세스 종료/재실행 후 수동 설정 30, 관측 ID, 4개 revision, 합계 10메소 복원.

검증용 앱과 Vite 서버는 확인 후 종료했다. 합성 데이터가 사용자 앱 데이터로 연결되지 않으며 원래 사용자 DB는 그대로다.

429·타임아웃·서버 오류와 빈/잘못된 응답은 기존 공통 전송 오류 테스트 및 scheduler 파서/세션 검사로 확인했다. 실제 서버에 오류를 유발하거나 Windows 시계를 변경하지 않았다. UI 로딩 상태의 실제 네트워크 대기 시간·실제 사용자 키의 성공 조회는 별도 실검증하지 않았다. release 빌드/패키징은 하지 않았다.

## 9. 남은 사항과 종료

- 명시적 date 요청의 400 원인은 확정하지 않았다. 현재는 성공 확인한 오늘 조회와 반환 날짜 검증으로 기능을 제공한다. 과거 API 조회는 이번 범위가 아니다.
- 미확인 가격 3개, 점검 당일의 정밀 가격 경계, 모든 특수 파티 예외는 기존 미검증 상태를 유지한다.
- 실제 사용자 키로 보스 화면의 성공 조회·실제 기록 수락을 추가 검증하려면 사용자 데이터에 영향을 주는 수락 전 내용을 확인해야 한다. 개발 검증에서 임의 수락하지 않았다.
- 다음 범위는 사용자 승인 후 결정한다. 메소·아이템 관리와 UI 리디자인은 시작하지 않았다. **7-C 완료 후 작업을 멈춘다.**
