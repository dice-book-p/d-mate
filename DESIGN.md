# D-Mate 전체 기획서

---

## 목차

1. [프로젝트 구성](#1-프로젝트-구성)
2. [D-Mate 앱](#2-d-mate-앱)
   - 2.1 [사이드바 구조](#21-사이드바-구조)
   - 2.2 [연결 관리 페이지](#22-연결-관리-페이지)
   - 2.3 [알림 규칙](#23-알림-규칙)
   - 2.4 [알림 상세 설계](#24-알림-상세-설계)
   - 2.5 [미연결 시 UX](#25-미연결-시-ux)
   - 2.6 [대시보드](#26-대시보드)
   - 2.7 [시스템 페이지](#27-시스템-페이지)
   - 2.8 [기술 스택](#28-기술-스택)
3. [D-Mate Desk 서버](#3-d-mate-desk-서버)
   - 3.1 [개요](#31-개요)
   - 3.2 [기술 스택](#32-기술-스택)
   - 3.3 [인증 체계](#33-인증-체계-참여코드)
   - 3.4 [기능 구분](#34-기능-구분)
   - 3.5 [MQTT 토픽 설계](#35-mqtt-토픽-설계)
   - 3.6 [Desk 미동작 시 D-Mate 동작](#36-desk-미동작-시-d-mate-동작)
   - 3.7 [서버 DB 스키마](#37-서버-db-스키마)
   - 3.8 [클라이언트 로컬 DB 추가](#38-클라이언트-로컬-db-추가)
   - 3.9 [API 설계](#39-api-설계)
   - 3.10 [관리자 대시보드](#310-관리자-대시보드-spa)
   - 3.11 [Docker Compose](#311-docker-compose)
   - 3.12 [프로젝트 디렉토리 구조](#312-프로젝트-디렉토리-구조)
   - 3.13 [보안 체크리스트](#313-보안-체크리스트)
4. [구현 로드맵](#4-구현-로드맵)

---

## 1. 프로젝트 구성

- **D-Mate**: 사내 업무 알림 데스크탑 앱 (Tauri v2 + Rust + Svelte 5)
  - macOS + Windows 크로스 플랫폼
  - GitHub Releases로 자동 업데이트 (tauri-plugin-updater)
  - 시스템 트레이 상주 (X 닫기 = 숨김, 트레이 종료 = 앱 종료)
- **D-Mate Desk**: 관리 서버 (FastAPI + Mosquitto MQTT + Docker Compose)
  - 기존 swork-update-server를 완전 대체 후 폐지
- 사용자 규모: 20~30명 사내 직원
- 1인 1디바이스 전제 (멀티 디바이스 시 중복은 사용자 책임)

---

## 2. D-Mate 앱

### 2.1 사이드바 구조

```
대시보드
연결 관리          (탭: SWORK | 메일 | Desk)
내 업무 알림        (작업자용)
관리 업무 알림      (지시자용)
메일 알림
메시지             (Desk Phase 3 - 메시징)
피드백             (Desk Phase 2 - 운영관리)
시스템
```

- 메시지/피드백은 Desk 구현 전까지 비표시 또는 placeholder
- 사이드바에 경고 뱃지 넣지 않음 (사용성 우선)

### 2.2 연결 관리 페이지

탭 구조로 구성한다.

#### SWORK 탭

- swork 계정 (아이디/비밀번호, 연결 확인, 해지)
- swork 텔레그램 봇 (봇 토큰/채팅ID, 테스트, 조회, 해지)

#### 메일 탭

- 메일 서버 (계정/서버/포트/SSL/비밀번호, 연결 확인, 해지)
- 메일 텔레그램 봇 (봇 토큰/채팅ID, 테스트, 조회, 해지)

#### Desk 탭

- Phase 0: "준비 중" placeholder 비활성 상태
- Phase 1 이후: 최초 시 참여코드 입력 폼 (서버주소, 참여코드, 이름, 디바이스명), 이후 연결 상태만 표시
- Desk는 별도 설정할 것 없이 연결 상태만 보여줌

### 2.3 알림 규칙

| 코드명 | 표시명 | 구분 | 설명 | API | dedup |
|--------|--------|------|------|-----|-------|
| my_overdue | 내 지연업무 알림 | 내 업무 | 내가 담당자인 업무 중 마감 초과 | ?type=worker | 일별 slot |
| my_deadline | 마감임박 알림 | 내 업무 | 내 업무 D-1, D-day | ?type=worker | 일별 (D-1/D-day 각각) |
| approval_request | 승인요청 알림 | 관리 업무 | 승인/검수 요청이 들어온 업무 | ?type=manager | 없음 (반복) |
| overdue_task | 지연업무 알림 | 관리 업무 | 내가 지시한 업무 중 마감 초과 | ?type=manager | 일별 slot |
| mail | 메일 알림 | 메일 | 새 메일 수신 | POP3 | 1회 (UID) |

#### 텔레그램 봇 구조

- swork용 봇 1개 (내 업무 + 관리 업무 공유)
- 메일용 봇 1개 (별도)
- 메시지 prefix로 구분: `[내 업무] ...` / `[관리] ...`

#### 각 규칙별 설정 항목

| 항목 | 설명 |
|------|------|
| enabled | on/off |
| schedule_type | "interval" 또는 "times" |
| interval_min | 분 단위 간격 |
| times | comma-separated HH:MM |
| use_work_hours | 근무시간만 여부 |

#### OS 네이티브 알림

- tauri-plugin-notification 사용
- macOS Notification Center / Windows Toast
- 텔레그램과 동시 발송
- 설정에서 on/off 토글

### 2.4 알림 상세 설계

#### swork API

- 기존: `GET /my-tasks/api/tasks?type=manager` (내가 지시한 업무)
- 신규: `GET /my-tasks/api/tasks?type=worker` (나에게 배정된 업무)

#### API 이중 호출 방지

- 통합 폴링 + 캐시 구조 (TaskCache)
- 한 폴링 주기에 manager 1회 + worker 1회 = 최대 2회
- 캐시된 데이터를 각 규칙이 필터링 (추가 API 호출 없음)
- 캐시 TTL: 폴링 주기와 동일 (최소 간격인 규칙 기준)

```rust
// TaskCache 구조 (의사코드)
struct TaskCache {
    manager_tasks: Vec<Task>,
    worker_tasks: Vec<Task>,
    fetched_at: DateTime,
}

// 폴링 시:
// 1. 캐시 만료 확인
// 2. 만료면 manager + worker 각 1회 fetch
// 3. 각 규칙은 캐시에서 필터링만
```

#### 내 지연업무 (my_overdue) 필터 로직

```rust
// 내가 담당자(t_assignee == username)이고 마감 초과된 업무
pub fn filter_my_overdue(tasks: &[Task], username: &str) -> Vec<Task> {
    let today = Local::now().date_naive();
    tasks.iter()
        .filter_map(|t| {
            if t.t_assignee != username { return None; }
            let status_ok = t.t_status == "업무승인"
                || t.t_status == "진행중"
                || t.t_status == "검수완료";
            if !status_ok { return None; }
            let due = parse_date(t.t_due_date.as_deref()?)?;
            if due >= today { return None; }
            let days = (today - due).num_days();
            let mut task = t.clone();
            task._days_overdue = Some(days);
            Some(task)
        })
        .collect()
}
```

- dedup: 일별 slot key (`my_overdue:{날짜}`)
- 관리자용 overdue_task와 동일한 로직이지만 `t_assignee == username` 필터 차이

#### 마감임박 (my_deadline) 필터 로직

```rust
// 내가 담당자이고 마감일이 D-1 또는 D-day인 업무
pub fn filter_my_deadline(tasks: &[Task], username: &str) -> Vec<Task> {
    let today = Local::now().date_naive();
    tasks.iter()
        .filter_map(|t| {
            if t.t_assignee != username { return None; }
            let status_ok = t.t_status == "업무승인" || t.t_status == "진행중";
            if !status_ok { return None; }
            let due = parse_date(t.t_due_date.as_deref()?)?;
            let days_left = (due - today).num_days();
            if days_left != 0 && days_left != 1 { return None; }
            let mut task = t.clone();
            task._days_left = Some(days_left); // 0=D-day, 1=D-1
            Some(task)
        })
        .collect()
}
```

- dedup slot key: `my_deadline:{t_code}:D-1:{날짜}` / `my_deadline:{t_code}:D-day:{날짜}`
- D-1과 D-day 각각 1회씩만 발송

#### 승인요청 (approval_request) 필터 로직 -- 기존 rule1

```rust
// 내가 지시한 업무 중 승인/검수 요청 상태
pub fn filter_approval_request(tasks: &[Task], username: &str) -> Vec<Task> {
    tasks.iter()
        .filter(|t| {
            t.t_assignee != username
                && (t.t_status == "승인요청" || t.t_status == "검수요청")
        })
        .cloned()
        .collect()
}
```

- dedup 없음 (처리될 때까지 매 주기마다 반복 알림)

#### 지연업무 (overdue_task) 필터 로직 -- 기존 rule2

```rust
// 내가 지시한 업무 중 마감 초과
pub fn filter_overdue_task(tasks: &[Task], _username: &str) -> Vec<Task> {
    // 기존 filter_rule2와 동일
}
```

- dedup: 일별 slot key (`overdue_task:{날짜}`)

#### 텔레그램 메시지 포맷

**내 지연업무:**

```
[내 업무] 지연 업무 알림

[TSK-001] API 문서 작성
  기한: 2026-03-24 (2일 초과)
  프로젝트: 다온플레이스 홈페이지
  상태: 진행중

[TSK-002] 테스트 케이스 작성
  기한: 2026-03-25 (1일 초과)
  프로젝트: 다온플레이스 홈페이지
  상태: 업무승인

총 2건
```

**마감임박:**

```
[내 업무] 마감 임박 알림

[TSK-003] 디자인 시안 작성 (D-day!)
  마감: 2026-03-26
  프로젝트: 브랜딩
  상태: 진행중

[TSK-004] 코드 리뷰 (D-1)
  마감: 2026-03-27
  프로젝트: 다온플레이스 홈페이지
  상태: 업무승인

총 2건
```

**승인/검수 요청:**

```
[관리] 승인/검수 요청 알림

[TSK-005] 홈페이지 수정
  프로젝트: 다온플레이스 홈페이지
  담당: 김철수 -> 박수용
  상태: 검수요청

총 1건
```

**지연업무 (관리):**

```
[관리] 지연 업무 알림

[TSK-006] 로고 디자인
  기한: 2026-03-20 (6일 초과)
  담당: 김철수 -> 박수용
  프로젝트: 브랜딩
  상태: 진행중

총 1건
```

### 2.5 미연결 시 UX

- 사이드바: 항상 깔끔 (경고 뱃지 없음)
- 알림 페이지 진입 시 필요 연결이 안 되어 있으면 상단 인라인 배너 표시
  - "SWORK 계정이 연결되지 않았습니다. 알림을 받으려면 먼저 연결이 필요합니다. [SWORK 연결하기 ->]"
  - "텔레그램 봇이 연결되지 않았습니다. [텔레그램 연결하기 ->]"
- 버튼 클릭 시 연결 관리 페이지의 해당 탭으로 이동
- 연결이 모두 완료되어 있으면 배너 없이 규칙 설정만 표시

### 2.6 대시보드

#### 카드 영역

| 카드 | 내용 |
|------|------|
| 내 지연업무 | N건 (내가 담당, 마감 초과) |
| 마감임박 | N건 (D-1, D-day) |
| 관리 요청 | N건 (승인/검수 요청) |
| 관리 지연 | N건 (내가 지시, 마감 초과) |

#### 목록 영역

- 내 업무 현황 (지연 + 마감임박, 긴급도 순 정렬)
- 승인/검수 요청 목록
- 관리 지연 업무 목록
- 최근 알림 로그

#### swork 업무 처리

- 대시보드/알림 페이지에서 직접 처리(승인/반려) 불가 (읽기 전용)
- 향후 확장 가능하도록 구조만 열어둠 (commands.rs에 placeholder)

#### 이스터에그

- 로고 7회 클릭 -> 컨페티
- 야근 감지 (대시보드 접근 시, 1시간 쿨다운)

### 2.7 시스템 페이지

- 자동시작 on/off
- 업데이트 확인 (tauri-plugin-updater, GitHub Releases)
- 데이터 초기화
- 근무시간 설정 (시작/종료 시간, 근무요일)

### 2.8 기술 스택

| 구분 | 기술 |
|------|------|
| 프레임워크 | Tauri v2 |
| 백엔드 | Rust (tokio, reqwest, rusqlite, keyring, native-tls, chrono, rumqttc) |
| 프론트엔드 | Svelte 5 (runes) |
| 빌드 | Vite |
| Tauri 플러그인 | tauri-plugin-updater, tauri-plugin-process, tauri-plugin-single-instance, tauri-plugin-autostart, tauri-plugin-notification |
| Keychain | keyring crate (apple-native, windows-native, sync-secret-service), 단일 JSON 엔트리 |
| CI/CD | GitHub Actions (macOS arm64, macOS x64, Windows x64) |

---

## 3. D-Mate Desk 서버

### 3.1 개요

D-Mate 앱을 지원하는 관리 서버. 기존 swork-update-server를 완전 대체한다.

### 3.2 기술 스택

| 구분 | 기술 |
|------|------|
| API 서버 | FastAPI (Python 3.12+), Uvicorn |
| MQTT 브로커 | Eclipse Mosquitto 2.x (TLS 8883) |
| DB | SQLite 3 (WAL) |
| 관리자 대시보드 | Svelte SPA |
| 인증 | JWT (HS256) + 참여코드 |
| DM 암호화 | X25519 (ECDH) + AES-256-GCM (DM 전용) |
| 전송 보안 | TLS 1.3 (REST + MQTT) |
| 배포 | Docker Compose (mosquitto + d-mate-desk) |

### 3.3 인증 체계 (참여코드)

**직원 가입 흐름:**

1. 관리자가 대시보드에서 참여코드 발급 (8자리 영숫자)
2. 직원이 D-Mate 앱에서 서버주소 + 참여코드 + 이름 + 디바이스명 입력
3. 서버가 코드 검증 후 JWT 발급 (30일) + refresh token

**참여코드 정책:**

- 영구 유효, 관리자 삭제/차단 시 무효화
- 같은 코드로 멀티 디바이스 등록 가능
- JWT 자동 갱신, refresh 만료 시 참여코드 재입력
- 차단: 코드 비활성화 시 해당 코드의 모든 디바이스 즉시 접속 차단

**MQTT 인증:**

- JWT 기반 username/password (Mosquitto auth plugin)

**관리자 인증:**

- 환경변수로 초기 admin 비밀번호 설정
- admin JWT 발급
- 모든 `/api/admin/*` 엔드포인트는 admin JWT 필수

### 3.4 기능 구분

#### A. 운영 관리 (우선순위 높음)

**1. 에러 리포팅**

- D-Mate 앱 오류 자동 수집
- 수집 항목: error_type, message, stack_trace, app_version, os, device_id
- 상태: new -> investigating -> resolved
- 보존: 90일 후 자동 삭제
- 민감 정보 마스킹 필터

**2. 피드백/건의사항 게시판**

- 카테고리: 버그 / 건의 / 질문
- 상태: open -> in_progress -> resolved / closed
- 관리자 답변 (admin_note) + 알림 발송
- 직원: 본인 글만 열람 / 관리자: 전체

**3. 접속/디바이스 관리**

- 디바이스 목록 (코드, 이름, OS, 버전, IP, 온라인 상태)
- 프레즌스: MQTT LWT (Last Will and Testament)
- 관리자: 개별 디바이스 또는 코드 단위 차단/삭제

**4. 통계**

- 버전 분포 (앱 버전별 사용자 수)
- 접속 현황 (온라인/오프라인, DAU 추이)
- 에러 추이 (일별 발생 건수)
- 메시지 통계 (공지/DM 발송 건수)
- OS 분포 (macOS / Windows)

#### B. 메시징 (운영 관리 이후)

**1. 공지 발송**

- 관리자 -> 전체 공지 (`d-mate/notice/all`)
- 관리자 -> 개인 알림 (`d-mate/notice/{code}`)

**2. DM (1:1 메시지)**

- DM은 desk API 경유 (`POST /api/dm` -> desk가 MQTT publish) -- 사칭 방지
- E2E 암호화: X25519 + AES-256-GCM (DM만)
- 공지/피드백/에러: TLS만 (관리자가 읽어야 하므로)
- 전달 상태: sent -> delivered -> read
- 오프라인 큐: 서버 offline_queue + 클라이언트 local_outbox

**3. 향후 확장**

- 그룹 채널
- 파일 전송
- 타이핑 표시

### 3.5 MQTT 토픽 설계

| 토픽 | 용도 | 발행자 | 구독자 | QoS |
|------|------|--------|--------|-----|
| d-mate/notice/all | 전체 공지 | desk | 모든 앱 | 1 |
| d-mate/notice/{code} | 개인 알림 | desk | 해당 사용자 | 1 |
| d-mate/dm/{code} | DM 수신함 | desk | 해당 사용자 | 2 |
| d-mate/presence/{code} | 온라인 상태 | 해당 앱 | desk + 관심 앱 | 0 |

**MQTT ACL:**

- 모든 사용자: 공지 구독 가능, 본인 토픽만 구독
- DM/공지 발행: desk 서버만
- 프레즌스: 본인만 발행, 전체 구독 가능

### 3.6 Desk 미동작 시 D-Mate 동작

| 기능 | 동작 |
|------|------|
| swork/메일 알림, 스케줄러, 설정 | 정상 작동 (독립) |
| 메시지/DM | "오프라인 모드" 표시, 로컬 outbox 저장 후 연결 시 자동 전송 |
| 피드백 | 로컬 저장 후 연결 시 자동 전송 |
| 에러 리포트 | 조용히 무시 |

**연결 상태 UI (Desk 탭 및 사이드바 하단):**

- 연결됨 (초록)
- 재연결 중... (노랑)
- 연결 끊김 (빨강)

### 3.7 서버 DB 스키마

```sql
-- 회원
CREATE TABLE members (
  code        TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  is_admin    INTEGER DEFAULT 0,
  is_active   INTEGER DEFAULT 1,
  created_at  TEXT DEFAULT (datetime('now'))
);

-- 디바이스
CREATE TABLE devices (
  device_id   TEXT PRIMARY KEY,
  code        TEXT NOT NULL REFERENCES members(code),
  device_name TEXT NOT NULL,
  os          TEXT,
  app_version TEXT,
  ip          TEXT,
  public_key  TEXT,
  is_online   INTEGER DEFAULT 0,
  last_seen   TEXT,
  is_active   INTEGER DEFAULT 1,
  created_at  TEXT DEFAULT (datetime('now'))
);

-- 대화방
CREATE TABLE conversations (
  id          TEXT PRIMARY KEY,
  type        TEXT NOT NULL,      -- dm | group
  created_at  TEXT DEFAULT (datetime('now'))
);

-- 대화방 멤버
CREATE TABLE conversation_members (
  conversation_id TEXT REFERENCES conversations(id),
  code            TEXT REFERENCES members(code),
  joined_at       TEXT DEFAULT (datetime('now')),
  PRIMARY KEY (conversation_id, code)
);

-- 메시지 (DM은 암호화 상태로 저장)
CREATE TABLE messages (
  id              TEXT PRIMARY KEY,
  conversation_id TEXT REFERENCES conversations(id),
  type            TEXT NOT NULL,    -- dm | notice | notify
  sender_code     TEXT,
  title           TEXT,
  body            TEXT,             -- 평문(공지) 또는 암호문(DM)
  ephemeral_key   TEXT,
  nonce           TEXT,
  priority        TEXT DEFAULT 'normal',
  created_at      TEXT DEFAULT (datetime('now'))
);

-- 전달 상태
CREATE TABLE message_delivery (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  message_id   TEXT REFERENCES messages(id),
  device_id    TEXT REFERENCES devices(device_id),
  status       TEXT DEFAULT 'sent',
  sent_at      TEXT DEFAULT (datetime('now')),
  delivered_at TEXT,
  read_at      TEXT
);

-- 오프라인 큐
CREATE TABLE offline_queue (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  message_id  TEXT,
  target_code TEXT,
  retry_count INTEGER DEFAULT 0,
  created_at  TEXT DEFAULT (datetime('now'))
);

-- 피드백
CREATE TABLE feedback (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  code        TEXT NOT NULL REFERENCES members(code),
  category    TEXT NOT NULL,        -- bug | suggestion | question
  title       TEXT NOT NULL,
  body        TEXT NOT NULL,
  status      TEXT DEFAULT 'open',  -- open | in_progress | resolved | closed
  admin_note  TEXT,
  created_at  TEXT DEFAULT (datetime('now')),
  updated_at  TEXT
);

-- 에러 리포트
CREATE TABLE error_reports (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id   TEXT,
  code        TEXT,
  error_type  TEXT NOT NULL,
  message     TEXT NOT NULL,
  stack_trace TEXT,
  app_version TEXT,
  os          TEXT,
  status      TEXT DEFAULT 'new',
  created_at  TEXT DEFAULT (datetime('now'))
);

-- 접속 로그
CREATE TABLE connection_log (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id   TEXT,
  code        TEXT,
  event       TEXT,                -- connect | disconnect
  ip          TEXT,
  created_at  TEXT DEFAULT (datetime('now'))
);
```

### 3.8 클라이언트 로컬 DB 추가

D-Mate 앱의 로컬 SQLite에 추가되는 테이블이다.

```sql
-- 복호화된 메시지 (Phase 3)
CREATE TABLE local_messages (
  id              TEXT PRIMARY KEY,
  conversation_id TEXT,
  type            TEXT,
  sender_code     TEXT,
  sender_name     TEXT,
  title           TEXT,
  body            TEXT,
  is_read         INTEGER DEFAULT 0,
  created_at      TEXT,
  received_at     TEXT DEFAULT (datetime('now'))
);

-- 암호화 키 (Phase 3)
CREATE TABLE encryption_keys (
  id          INTEGER PRIMARY KEY,
  private_key BLOB NOT NULL,
  public_key  TEXT NOT NULL,
  created_at  TEXT DEFAULT (datetime('now'))
);

-- 미전송 메시지 (Phase 3)
CREATE TABLE local_outbox (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  target_code     TEXT NOT NULL,
  encrypted_body  TEXT NOT NULL,
  ephemeral_key   TEXT NOT NULL,
  nonce           TEXT NOT NULL,
  created_at      TEXT DEFAULT (datetime('now'))
);

-- Desk 연결 정보 (Phase 1)
CREATE TABLE desk_config (
  key   TEXT PRIMARY KEY,
  value TEXT
);
```

### 3.9 API 설계

#### 인증

| 메서드 | 경로 | 설명 |
|--------|------|------|
| POST | /api/auth/join | 참여코드 가입 + JWT 발급 |
| POST | /api/auth/refresh | JWT 갱신 |
| POST | /api/admin/login | 관리자 로그인 |

#### 메시징

| 메서드 | 경로 | 설명 |
|--------|------|------|
| POST | /api/dm | DM 발송 |
| POST | /api/dm/{msg_id}/delivered | 전달 확인 |
| POST | /api/dm/{msg_id}/read | 읽음 확인 |
| GET | /api/dm/conversations | 대화 목록 |
| GET | /api/dm/conversations/{id}/messages | 대화 메시지 |
| GET | /api/contacts | 사용자 목록 |
| GET | /api/contacts/{code}/public-key | 공개키 조회 |

#### 관리자 공지

| 메서드 | 경로 | 설명 |
|--------|------|------|
| POST | /api/admin/notice | 전체 공지 |
| GET | /api/admin/notice | 공지 이력 |
| POST | /api/admin/notify/{code} | 개인 알림 |

#### 회원 관리

| 메서드 | 경로 | 설명 |
|--------|------|------|
| POST | /api/admin/members | 참여코드 생성 |
| GET | /api/admin/members | 회원 목록 |
| PATCH | /api/admin/members/{code} | 수정/차단 |
| DELETE | /api/admin/members/{code} | 삭제 |

#### 디바이스

| 메서드 | 경로 | 설명 |
|--------|------|------|
| GET | /api/admin/devices | 목록 |
| PATCH | /api/admin/devices/{id} | 활성/비활성 |
| DELETE | /api/admin/devices/{id} | 삭제 |

#### 피드백

| 메서드 | 경로 | 설명 |
|--------|------|------|
| POST | /api/feedback | 제출 |
| GET | /api/feedback | 내 목록 |
| GET | /api/admin/feedback | 전체 목록 |
| PATCH | /api/admin/feedback/{id} | 상태/답변 |

#### 에러

| 메서드 | 경로 | 설명 |
|--------|------|------|
| POST | /api/errors | 에러 전송 |
| GET | /api/admin/errors | 목록 |
| PATCH | /api/admin/errors/{id} | 상태 변경 |

#### 통계

| 메서드 | 경로 | 설명 |
|--------|------|------|
| GET | /api/admin/stats/overview | 전체 요약 |
| GET | /api/admin/stats/versions | 버전 분포 |
| GET | /api/admin/stats/connections | 접속 현황 |
| GET | /api/admin/stats/errors | 에러 추이 |
| GET | /api/admin/stats/messages | 메시지 통계 |

#### 헬스

| 메서드 | 경로 | 설명 |
|--------|------|------|
| GET | /api/health | 서버 상태 확인 |

### 3.10 관리자 대시보드 (SPA)

```
/admin
  /dashboard        통계 요약
  /members          참여코드 관리
  /devices          디바이스 목록
  /messaging
    /notice         공지 발송 + 이력
    /notify         개인 알림
  /errors           에러 리포트
  /feedback         피드백 관리
  /settings         서버 설정
```

### 3.11 Docker Compose

```yaml
version: "3.8"
services:
  mosquitto:
    image: eclipse-mosquitto:2
    ports:
      - "8883:8883"
    volumes:
      - ./mosquitto/config:/mosquitto/config:ro
      - ./mosquitto/data:/mosquitto/data
      - ./certs:/mosquitto/certs:ro
    restart: unless-stopped

  d-mate-desk:
    build: .
    ports:
      - "8800:8800"
    environment:
      - ADMIN_PASSWORD=${ADMIN_PASSWORD}
      - JWT_SECRET=${JWT_SECRET}
      - MQTT_HOST=mosquitto
      - MQTT_PORT=1883
      - DATABASE_PATH=/data/d-mate-desk.db
    volumes:
      - ./data:/data
    depends_on:
      - mosquitto
    restart: unless-stopped
```

### 3.12 프로젝트 디렉토리 구조

```
d-mate-desk/
  docker-compose.yml
  Dockerfile
  .env
  requirements.txt
  app/
    main.py
    config.py
    auth.py
    database.py
    mqtt_service.py
    models/
    routers/
    static/admin/
  dashboard/              (Svelte SPA)
    package.json
    src/
  mosquitto/
    config/
      mosquitto.conf
      acl.conf
      passwd
  certs/
  data/
```

### 3.13 보안 체크리스트

1. MQTT TLS (8883) - 사내 자체 서명 CA, 앱에 CA cert 내장
2. REST HTTPS
3. DM E2E (X25519 + AES-256-GCM) - 서버 복호화 불가
4. MQTT ACL - 토픽별 접근 제어
5. JWT 서명 검증
6. DM API 경유 - 사칭 방지
7. 참여코드 차단 시 즉시 무효화
8. 참여코드 브루트포스 방어 (rate limit)
9. 입력 검증 (Pydantic)
10. 에러 리포트 민감정보 마스킹
11. 환경변수 관리 (.env, git 미포함)

---

## 4. 구현 로드맵

### Phase 0: D-Mate 작업자 알림 + UI 개편 (v1.1.0) -- 최우선

#### 0-1. 네이밍 전환 + DB 마이그레이션

- 기존 `rule1` -> `approval_request`, `rule2` -> `overdue_task` 전면 리네이밍
- `notification_log` 기존 데이터 마이그레이션: `UPDATE notification_log SET rule_type='approval_request' WHERE rule_type='rule1'`
- Settings 구조체/DB 칼럼 변경:
  - 기존: `rule1_enabled`, `rule1_schedule_type`, `rule1_interval_min`, `rule1_times`, `rule1_use_work_hours`
  - 변경: `approval_request_enabled`, `approval_request_schedule_type`, ...
  - 동일하게 `rule2_*` -> `overdue_task_*`
  - 신규 추가: `my_overdue_*`, `my_deadline_*` (각각 enabled/schedule_type/interval_min/times/use_work_hours)

#### 0-2. 백엔드 (Rust) - swork 작업자 알림

- `swork_client.rs`:
  - `fetch_worker_tasks()` 추가 (`GET /my-tasks/api/tasks?type=worker`)
  - `TaskCache` 구조체 도입 (manager + worker 캐싱, TTL 기반)
- `notification_rules.rs`:
  - `filter_my_overdue(tasks, username)` 추가 -- 내 업무 중 마감 초과
  - `filter_my_deadline(tasks, username)` 추가 -- 내 업무 D-1, D-day
  - `format_my_overdue_message(tasks)` 추가 -- `[내 업무] 지연 업무 알림` 포맷
  - `format_my_deadline_message(tasks)` 추가 -- `[내 업무] 마감 임박 알림` 포맷
  - 기존 `filter_rule1` -> `filter_approval_request`, `format_rule1_message` -> `format_approval_request_message`
  - 기존 `filter_rule2` -> `filter_overdue_task`, `format_rule2_message` -> `format_overdue_task_message`
  - 기존 메시지에 `[관리]` prefix 추가
- `models.rs`:
  - Task 구조체에 `_days_left: Option<i64>` 필드 추가 (마감임박용)
  - Settings에 `my_overdue_*`, `my_deadline_*` 필드 추가 (각 5개)
  - DashboardData에 `my_overdue_tasks: Vec<Task>`, `my_deadline_tasks: Vec<Task>` 추가
- `checker.rs`:
  - `check_my_overdue()` 추가 -- worker tasks 필터 -> 텔레그램 + OS 알림
  - `check_my_deadline()` 추가 -- D-1/D-day 필터 -> 텔레그램 + OS 알림
  - 기존 `check_rule1()` -> `check_approval_request()` 리네이밍
  - 기존 `check_rule2()` -> `check_overdue_task()` 리네이밍
- `scheduler.rs`:
  - my_overdue, my_deadline 스케줄 루프 추가
  - 기존 rule1/rule2 루프 리네이밍
- `database.rs`:
  - Settings 칼럼 마이그레이션 (ALTER TABLE + UPDATE)
  - notification_log rule_type 마이그레이션
- `commands.rs`:
  - `get_dashboard_data` 확장 (worker tasks 포함)

#### 0-3. 백엔드 (Rust) - OS 네이티브 알림

- `Cargo.toml`에 `tauri-plugin-notification` 추가
- `lib.rs`에 `.plugin(tauri_plugin_notification::init())` 추가
- `capabilities/default.json`에 notification 권한 추가
- `checker.rs`의 각 check 함수에서 텔레그램 발송과 동시에 OS 알림 발송
- `models.rs` Settings에 `os_notification_enabled: i32` 추가 (on/off 토글)

#### 0-4. 프론트엔드 (Svelte) - UI 전면 개편

**사이드바 재구성 (8개 메뉴):**
- 대시보드, 연결 관리, 내 업무 알림, 관리 업무 알림, 메일 알림, 메시지(placeholder), 피드백(placeholder), 시스템
- `Sidebar.svelte` 전면 재작성

**연결 관리 페이지 (신규):**
- `ConnectionManager.svelte` 신규 생성
- 탭 구조: SWORK / 메일 / Desk
- 기존 `ServiceSwork.svelte`에서 연결 설정(계정+텔레그램) 분리 이동
- 기존 `ServiceMail.svelte`에서 연결 설정(서버+텔레그램) 분리 이동
- Desk 탭은 "준비 중" placeholder 비활성 상태
- 탭마다 연결 상태 뱃지 표시 (초록/빨강)

**내 업무 알림 페이지 (신규):**
- `WorkerAlerts.svelte` 신규 생성
- 내 지연업무 규칙 설정 블록 (enabled, schedule_type, interval/times, use_work_hours)
- 마감임박 규칙 설정 블록 (동일 구조)
- 미연결 시 상단 인라인 배너 + [SWORK 연결하기 ->] / [텔레그램 연결하기 ->]
- dirty check + [저장] 버튼

**관리 업무 알림 페이지 (기존 리팩토링):**
- `ManagerAlerts.svelte` (기존 ServiceSwork에서 규칙 설정 부분만 추출)
- 승인요청 규칙 설정 블록
- 지연업무 규칙 설정 블록
- 미연결 배너 동일

**메일 알림 페이지 (기존 리팩토링):**
- `MailAlerts.svelte` (기존 ServiceMail에서 규칙 설정 부분만 추출)
- 메일 알림 규칙 설정 블록
- 미연결 배너 동일

**대시보드 개편:**
- `Dashboard.svelte` 카드 4개: 내 지연업무, 마감임박, 관리 요청, 관리 지연
- 목록: 내 업무 현황(지연+임박), 승인/검수 요청, 관리 지연, 최근 로그

**라우팅/스토어:**
- `lib/stores.js`: currentPage에 'connection', 'worker_alerts', 'manager_alerts', 'mail_alerts' 추가
- `lib/api.js`: 기존 invoke 함수명은 유지 (백엔드 command명과 매칭)
- `App.svelte`: 라우터 분기 업데이트

### Phase 1: Desk 서버 기반 구축 (2주)

- FastAPI + SQLite + Docker 환경
- DB 스키마 생성
- 참여코드 인증 + JWT
- Mosquitto 설정 (TLS, ACL)
- D-Mate 앱에 Desk 연결 기능 활성화 (연결 관리 Desk 탭)
- 기존 update-server 에러 리포팅 이전

### Phase 2: 운영 관리 (1주) -- 메시징보다 우선

- 에러 리포팅 API + 대시보드
- 피드백 게시판 API + 앱 UI + 대시보드
- 접속/디바이스 관리 API + 대시보드
- 통계 API + 대시보드 차트

### Phase 3: 메시징 (3주)

- 공지 발송 (REST -> MQTT)
- DM API + MQTT 라우팅
- E2E 암호화 (X25519 + AES-256-GCM)
- DM UI (대화 목록, 채팅 화면, 연락처)
- 오프라인 큐 (서버 + 클라이언트)
- 전달 상태 (sent/delivered/read)

### Phase 4: 관리자 대시보드 + 안정화 (2주)

- Svelte SPA 관리 페이지 전체 구현
- 통합 테스트
- 보안 점검
- TLS 인증서 운영 적용
- D-Mate 앱 신규 버전 배포
