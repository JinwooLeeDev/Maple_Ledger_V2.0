# Maple Ledger

Tauri 2 + React + TypeScript + Rust 기반 Windows 데스크톱 앱입니다. API 키 설정, 본캐·관리 선택과 최대 6개 캐릭터 대시보드를 제공합니다.

## 개발 실행

Node.js/npm, Rust MSVC 툴체인, Visual Studio C++ 개발 도구 및 WebView2가 필요합니다. 사용자 PATH에는 `%USERPROFILE%\.cargo\bin`이 포함되어야 합니다. PATH 변경 전부터 열려 있던 터미널이나 편집기는 재시작하세요.

```powershell
npm ci
npm run tauri dev
```

이미 의존성이 설치되어 있으면 `npm ci`를 반복할 필요가 없습니다. 첫 개발 실행에는 Rust 의존성 다운로드와 컴파일이 필요하며 다음 실행에서는 Cargo 캐시를 재사용합니다.

```powershell
npm run typecheck  # React와 Vite 설정 타입 검사, 빌드 없음
npm run dev        # 프런트엔드만 실행
```

`npm run build`는 프런트엔드 산출물을 생성하는 명령입니다. 배포 패키징은 현재 비활성화했으며 10단계에서 구성합니다. 키 설정과 캐릭터 조회는 Windows 앱에서 사용하세요. 브라우저만 실행한 경우 Rust IPC를 사용할 수 없습니다.

설정에서 본인 API 키를 저장한 뒤 **캐릭터 관리 → 캐릭터 목록 불러오기**를 누르세요. 키 저장 시점에는 유효성을 검사하지 않으며 실제 조회 성공 시 API 접근을 확인할 수 있습니다. 목록은 계정·월드별로 표시하고 전체 계정 ID와 OCID는 화면에 노출하지 않습니다.

메인 대시보드는 SQLite의 관리 선택을 먼저 표시하고 저장된 OCID로 기본 정보와 이미지를 갱신합니다. 본캐가 첫 카드이고 나머지는 저장 순서입니다. 전체 목록을 다시 조회하지 않습니다. **카드 새로고침**으로 갱신하거나 설정·캐릭터 관리 화면으로 이동할 수 있습니다. 개별 조회 실패 시 기존 정보를 유지하고 이미지가 없으면 대체 이미지를 표시합니다.

**전체 캐릭터에서 본캐 선택·변경** 영역에서 본캐 1개를 지정하면 동일 계정·월드의 후보를 표시합니다. **관리 캐릭터 선택**에서 추가 5개까지 선택하고 저장하세요. 본캐는 항상 고정 포함되며 해제할 수 없습니다. 6개에 도달하면 새 선택이 제한되고 추가 항목 해제는 계속 가능합니다. 선택한 순서를 저장하고 재실행 시 복원합니다.

같은 계정·월드의 본캐 변경은 새 본캐를 고정하고 기존 추가 선택을 재검사해 유지합니다. 이전 본캐를 자동으로 추가하지 않습니다. 다른 계정/월드 변경은 확인 후 추가 선택을 초기화하며 취소 시 이전 설정을 유지합니다. 본캐/선택 해제로 보스 기록을 삭제하지 않습니다. 주간 보스 수익을 지원하며 일반 메소 잔액·아이템 관리는 아직 없습니다.

조회에서 사라진 기존 선택은 삭제하지 않고 상태를 표시합니다. 조회 전에도 기존 추가 선택을 해제하여 저장할 수 있지만 새 캐릭터 추가에는 정상 목록 확인이 필요합니다. 체크박스 변경은 **관리 선택 저장**을 눌러야 반영됩니다. 본캐 변경이나 목록 갱신 전에는 원하는 변경을 먼저 저장하세요.

## 구조

```text
src/main.tsx                 React 진입점
src/app/                     기본 화면·스타일
src/features/settings/       API 키 설정 화면
src/features/characters/     캐릭터 목록 화면
src/features/dashboard/      메인 대시보드·캐릭터 카드
src/shared/api/              타입화한 Tauri 호출
src-tauri/src/               Rust 진입점과 Tauri 실행
src-tauri/src/commands/      키 관리·캐릭터 목록 IPC
src-tauri/src/nexon/         목록·기본 정보·공개 이미지 클라이언트
src-tauri/src/services/      요청 직렬화·호출 간격·429 대기
src-tauri/src/domain/        OCID 기반 본캐 식별·계정/월드 필터
src-tauri/src/storage/       본캐·관리 선택 SQLite 저장소
src-tauri/migrations/        버전 순서의 SQL 마이그레이션
src-tauri/src/security/      Windows Credential Manager 어댑터와 테스트
src-tauri/capabilities/      main 창의 키·목록·본캐·관리 선택 IPC만 허용
src-tauri/icons/             기본 개발 아이콘
src-tauri/tauri.conf.json    com.mapleledger.app 및 창·개발 서버 설정
docs/                       API 레퍼런스·설계·단계별 개발 계획
secrets/                    개발 전용, Git 및 프런트엔드 접근 제외
public/avatar-placeholder.svg  로컬 대체 이미지
```

본캐와 관리 선택은 SQLite에 저장합니다. 전체 API 목록은 메모리에서만 유지하고 재실행 후 다시 조회합니다. 필터·선택 조건·중복·개수 검증은 Rust에서 수행합니다. 본캐와 관리 선택 변경은 원자적으로 처리합니다. 전체 계층 계획은 [아키텍처](docs/ARCHITECTURE.md), 승인 단위는 [개발 계획](docs/DEVELOPMENT_PLAN.md)을 따릅니다.

카드 기본 정보·이미지는 앱 실행 중 메모리에만 보관합니다. 정상 결과는 15분 재사용하고 수동 새로고침은 갱신하되 2초 이내 반복은 합칩니다. 같은 이미지 URL은 재사용합니다. 재실행 시 저장된 표시 정보와 대체 이미지를 먼저 보여 주고 다시 조회합니다. 카드 조회는 DB 관리 선택과 보스 기록을 변경하지 않습니다.

## 데이터와 비밀정보

개발 키는 `secrets/nexon-api-key.txt`에 유지하며 명시적으로 실행한 Rust 실연동 테스트에서만 읽습니다. 앱은 해당 파일을 읽거나 사용자 보안 저장소에 자동 등록하지 않으며 배포에도 포함하지 않습니다. Vite는 루프백에서만 실행하고 secrets 경로 제공을 차단합니다. SQLite 파일 및 sidecar, node_modules, dist, Cargo target과 생성 스키마는 Git에서 제외합니다. package-lock.json과 Cargo.lock은 재현 가능한 설치를 위해 유지합니다.

사용자 키는 Windows Credential Manager의 `com.mapleledger.app/nexon-open-api` 일반 자격 증명에 Local 지속성으로 저장합니다. 앱 재실행 후에도 유지되며 저장 원문을 UI로 가져오는 명령은 없습니다. 설정에서 저장하면 기존 키가 교체되고 삭제하면 이 앱의 키만 제거됩니다. 실제 목록 조회 때 Rust가 키를 읽어 API에 전달합니다.

DB는 Tauri의 사용자 앱 로컬 데이터 디렉터리 아래 `ledger.sqlite3`에 저장합니다. Windows 경로는 `%LOCALAPPDATA%\com.mapleledger.app\ledger.sqlite3`입니다. 실행 파일/리포지토리와 분리하고 API 키는 포함하지 않습니다. v2의 본캐·관리 선택·순서를 보존하며 v3는 보스 계획·기간·클리어/revision을 추가합니다. 일반 메소·아이템 테이블은 아직 없습니다.

본캐 관련 최소 검증은 실제 API 호출 없이 실행합니다. 테스트 DB는 고유한 임시 파일을 사용하고 종료 시 정리합니다.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib domain:: -- --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml --lib storage:: -- --test-threads=1
```

키 저장 계층의 관련 검증은 다음과 같이 실행합니다. 합성 키를 고유한 테스트 자격 증명에 저장하고 테스트 종료 시 삭제하며 앱의 실제 등록 키는 건드리지 않습니다.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib security::tests -- --test-threads=1
```

API 클라이언트와 서비스의 일반 테스트는 실호출 없이 실행합니다. 실연동 테스트는 기본적으로 ignored 상태이며 아래 마지막 명령을 명시적으로 실행할 때만 개발 키로 요청 1회를 보냅니다. 응답은 메모리에서만 검증하고 로그에는 집계 및 필드명만 기록합니다.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib nexon::tests -- --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml --lib services::tests
# 필요할 때만 실행: 실제 요청 1회
cargo test --manifest-path src-tauri/Cargo.toml --lib nexon::tests::live_character_list_schema -- --exact --ignored --nocapture
```

대시보드의 관련 테스트는 별도 브라우저나 앱 설치 없이 실행합니다.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib nexon:: -- --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml --lib services:: -- --test-threads=1
node scripts/check-dashboard.mjs
```

실행 중 요청 횟수를 확인할 때만 `MAPLE_LEDGER_REQUEST_COUNTS=1` 환경 변수를 사용할 수 있습니다. debug 빌드에서 기본 정보·공개 이미지의 누적 횟수만 출력하며 URL·식별자·키는 출력하지 않습니다. 기본 실행 및 release에는 이 출력을 남기지 않습니다.

작업 결과와 검증 한계는 [6단계 완료 기록](docs/STAGE_6_REPORT.md)과 [7-B 완료 기록](docs/STAGE_7B_REPORT.md)에 있습니다.

### 주간 보스 관리

캐릭터 카드의 **보스 관리**에서 보스·난이도를 선택하고 설정을 저장합니다. 파티 인원(기본 1명)과 클리어를 입력한 뒤 **기록 저장**을 누르면 해당 주차 수익이 반영됩니다. 캐릭터/주차 이동 전에 편집 내용을 저장해 주세요.

개인 수익은 1인 기준 결정석 가격을 파티 인원으로 나눈 정수 몫입니다. 결정석은 모두 판매한다고 가정하며 실제 잔액·판매일은 관리하지 않습니다. 미확인 가격/적용 시점은 미산정으로 표시합니다. 수동 가격 입력은 제공하지 않습니다. 과거 사용자 확인 가격으로 저장된 기록은 당시 스냅샷을 유지하며 자료 갱신으로 자동 변경되지 않습니다.

주차는 목요일 00:00 KST 기준입니다. 이전 주차, 클리어 해제/정정 및 수정 이력을 지원합니다. 보스 추적 또는 캐릭터 관리 선택을 해제해도 과거 기록은 삭제하지 않습니다. 스키마는 v4이며 앱 데이터 경로는 유지됩니다. 현재 관리 대상 외 기록은 별도 접힘 영역에 보존합니다.

관련 검증만 실행하려면:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib bosses --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib storage --offline
node scripts/check-bosses.mjs
```

개발 전용 UI 점검 시 `MAPLE_LEDGER_CHECK_DIR`(기존 OS 임시 하위 폴더)는 분리 DB를 사용하고, `MAPLE_LEDGER_OFFLINE_CHECK=1`은 카드의 API 갱신을 차단합니다. 기본 사용에는 설정하지 않으며 release에서는 적용되지 않습니다. UI fixture 준비 테스트는 명시적으로 실행하는 ignored 테스트입니다.

스케줄러 등록 여부와 완료 여부는 별개로 표시합니다. API는 보스 화면에서 사용자가 명시적으로 조회하며 기존 관리 목록이나 수동 기록을 자동 변경하지 않습니다. 수동 보스 선택은 유지합니다. 자동 목록 구성의 검증 상태는 [7-D 보고](docs/STAGE_7D_REPORT.md)를 확인하세요.
