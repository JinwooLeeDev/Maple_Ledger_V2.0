# 초기 조사 및 API 분석

조사일: 2026-10-28. 이번 작업은 조사와 문서 작성만 수행했다. 실제 API 호출은 0회이며 아래 응답 구조는 제공된 명세 기준이다. 실응답과의 일치 여부는 아직 검증하지 않았다.

## 환경 및 파일

- 루트: `C:\Users\10203\OneDrive\Desktop\Maple Ledger`
- 기존 파일: `.gitignore`, `docs/NEXON_MAPLESTORY_API_REFERENCE.md`, `secrets/nexon-api-key.txt`, `.git/`.
- 최초 Git 작업 트리는 깨끗했다. 기존 추적 파일은 `.gitignore`와 API 레퍼런스뿐이다. 조회된 커밋은 `bf52d77 first commit`이다.
- 키 파일은 존재하며 내용은 읽거나 출력하지 않았다. `.gitignore`의 `/secrets/`가 적용된다. `.env`, `.env.*`도 제외하고 `.env.example`은 예외 처리되어 있다.
- 현재 추적 파일, 전체 참조의 secrets 경로 및 키 이름·환경 파일 경로 이력 조회에서는 비밀 파일 포함 정황이 없었다. 삭제된 Git 객체나 외부 복제본까지 조사한 보증은 아니다.
- Node.js v22.15.0, npm 10.9.2 확인.
- 현재 셸 PATH에서는 rustc/cargo/rustup을 찾지 못했다. `%USERPROFILE%/.cargo/bin/`의 실행 파일로 Rust 1.99.1, Cargo 1.99.1 및 설치 타깃 `x86_64-pc-windows-msvc`를 확인했다. 재설치보다 다음 단계의 PATH 점검을 우선한다.
- vswhere로 C++ 도구 구성 요소를 갖춘 Visual Studio 2022 Community 설치를 확인했다. 실제 링크 및 Windows SDK 동작은 빌드하지 않아 미검증이다.
- 레지스트리에서 WebView2 Runtime 155.0.4283.62 확인. 실제 앱 렌더링은 미검증이다.
- package.json, Cargo.toml, Tauri 설정과 로컬 Tauri CLI는 아직 없다. 전역 tauri 명령도 발견하지 못했다. Node/npm 버전의 향후 템플릿 호환성은 초기화 시 선택 버전 기준으로 확인한다.
- 설치, 프로젝트 생성, 기능 구현, 빌드, 커밋 및 푸시는 수행하지 않았다.

Windows의 C++ 도구 및 WebView2 요구사항은 [Tauri 2 공식 사전 준비 문서](https://v2.tauri.app/start/prerequisites/)를 참고했다.

## 명세 구조

제공 문서는 생성 시각 2026-10-28 UTC, 공식 명세 기반의 생성 레퍼런스이며 실응답 검증 자료가 아님을 명시한다. 11개 카테고리와 63개 작업 항목에는 API 키와 Open ID의 동일 HTTP 경로가 중복 등장한다. 인증 방식까지 구분해야 한다.

## 캐릭터 목록

`GET https://open.api.nexon.com/maplestory/v1/character/list`

- 필수 헤더: `x-nxopen-api-key`. API 키 방식에는 쿼리 파라미터와 요청 본문이 없다.
- 응답 루트 `account_list`는 메이플스토리 계정 배열이다.
- 각 계정은 `account_id: string`, `character_list: array`를 가진다.
- 각 캐릭터는 `ocid: string`, `character_name: string`, `world_name: string`, `character_class: string`, `character_level: number (int64)`를 가진다.
- 목록 항목에 `account_id`를 임의로 있다고 가정하지 않고 부모 계정에서 내부 모델로 전달한다.
- 본캐릭터와 부모 `account_id` 및 `world_name`이 모두 일치하는 캐릭터를 선택 후보로 만들 수 있다. 닉네임과 직업 추측이나 추가 기본 정보 요청은 필요하지 않다.
- 본캐는 전체 목록에서 선택한다. 키가 조회할 수 있는 목록을 전체 게임 사용자 목록으로 확대 해석하지 않는다.
- 스키마에 필수 필드 목록이 명확히 지정되지 않은 부분은 방어적으로 처리한다. 계정/월드 식별 정보 누락 시 추측으로 후보에 포함하지 않고 오류나 재조회 안내를 제공한다.
- Open ID 카테고리의 같은 경로는 `Authorization: Bearer ...`와 클라이언트 활용 데이터 설정을 요구한다. 이번 API 키 방식에 도입하지 않는다.

## 기본 정보와 이미지

`GET /maplestory/v1/character/basic`에 필수 `ocid` 쿼리와 API 키 헤더를 보낸다. 선택 `date`는 KST `YYYY-MM-DD`다. `CharacterBasic.character_image`가 외형 이미지 URL이다. 이미지가 필요한 관리 대상에 한해 요청하고 캐시한다. 목록 단계에서 모든 캐릭터의 기본 정보를 추가 요청하지 않는다. 닉네임→OCID 변환용 `/maplestory/v1/id`는 목록에 OCID가 이미 있어 불필요하다.

명세의 일반 안내는 평균 약 15분의 게임 데이터 반영 지연, 과거 일자 조회와 전일 데이터 익일 오전 2시 제공, 게임 콘텐츠 변경으로 OCID가 변경될 수 있음을 설명한다. 이를 모든 API의 동일한 실시간 보장으로 해석하지 않는다. OCID 변경 시 자동으로 닉네임만 대조하여 장부를 재연결하지 않는다.

## 오류와 검증

관련 API는 400, 403, 429, 500과 `error: { name: string, message: string }`를 명세한다. 구체적 오류 코드 목록을 임의로 만들지 않는다. 400은 입력/날짜 확인, 403은 키 및 접근 권한 확인, 429는 요청 중지와 지연 재시도, 500은 제한된 재시도와 캐시 표시 대상으로 설계한다. 미명세 HTTP 상태, 비JSON 응답, 빈 데이터, 스키마 불일치, 네트워크 단절과 타임아웃도 안전하게 처리한다. 인증 실패는 자동 반복하지 않는다. Retry-After가 실제 제공되면 존중하되 제공된다고 가정하지 않는다.

실연동 단계에서는 목록을 한 번 요청하여 상태와 필드명·자료형·계정 배열 관계만 검증하고 개인 식별 정보와 키를 보고서에 출력하지 않는다. 응답 원문을 Git에 저장하지 않는다. 이번 단계는 명세로 분석 가능하여 실호출을 생략했다.

## API 지원 범위와 한계

- 명세에 `GET /maplestory/v1/scheduler/character-state`가 있다. 헤더는 API 키, 쿼리는 자신의 계정 캐릭터의 `ocid`와 선택 `date`다.
- 스케줄러는 오늘 및 요청일 기준 최대 14일 전까지 조회 가능하다. 해당 기준일 미접속 시 결과가 없을 수 있다. 보스 관련 정보는 콘텐츠 완료 시 갱신된다고 설명한다.
- `boss_contents[]`에는 `content_name`, `difficulty`, `cycle`, `list_order_no`, `registration_flag`, `complete_flag`가 있다. 두 flag는 boolean이 아닌 문자열이다. `weekly_boss_clear_count`, `weekly_boss_clear_limit_count`도 제공한다.
- 따라서 보스 완료 상태의 보조 동기화는 검토 가능하다. 빈 결과를 미클리어로 단정하거나 이것만으로 영구 이력을 복구하지 않는다. 수동 기록과 API 관측값의 우선순위는 7단계에서 결정한다.
- 해당 스케줄러 응답에는 결정석 가격, 실제 판매 금액, 수령 메소, 파티 분배값이 없다. `cycle`만으로 정확한 초기화 시각도 확정할 수 없다.
- 제공된 엔드포인트 목록에는 월드 메소 잔액, 아이템 전체 획득·판매 장부 및 수익 원장을 직접 완성하는 API가 없다. 장착 장비나 강화 이력을 판매·인벤토리 이력으로 대체할 수 없다.
- 본캐 지정, 최대 6개 선택, 사용자 장부, 장기 이력, 통계 및 앱 종료 중 초기화 처리는 로컬 기능이다. 보상과 초기화 규칙은 공식 근거 및 사용자 승인 후 정의한다.
