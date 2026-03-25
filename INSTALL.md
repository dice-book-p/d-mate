# D-Mate 설치 가이드

## 다운로드

| OS | 파일 |
|------|------|
| macOS (Apple Silicon) | `D-Mate_x.x.x_aarch64.dmg` |
| macOS (Intel) | `D-Mate_x.x.x_x64.dmg` |
| Windows | `D-Mate_x.x.x_x64-setup.exe` |

---

## macOS 설치

### 1. DMG 파일 열기
다운로드한 `.dmg` 파일을 더블클릭합니다.

### 2. Applications 폴더로 복사
D-Mate.app을 Applications 폴더로 드래그합니다.

### 3. 보안 허용 (최초 1회)
코드서명이 없는 앱이므로 macOS가 차단합니다. **터미널**에서 아래 명령어를 실행하세요:

```bash
xattr -cr /Applications/D-Mate.app
```

> 터미널 여는 법: Spotlight(⌘+Space) → "터미널" 검색 → 실행

### 4. 앱 실행
Applications에서 D-Mate를 더블클릭합니다.

### 5. Keychain 접근 허용
최초 실행 시 Keychain 접근 허용 팝업이 뜹니다. **"항상 허용"**을 선택하세요.

---

## Windows 설치

### 1. 설치 파일 실행
다운로드한 `.exe` 파일을 더블클릭합니다.

### 2. SmartScreen 경고 우회
"Windows가 PC를 보호했습니다" 화면이 나타나면:
1. **"추가 정보"** 클릭
2. **"실행"** 클릭

### 3. 설치 완료
설치 마법사의 안내에 따라 진행합니다.

---

## 초기 설정

### SWORK 알림
1. **SWORK 알림** 메뉴 → swork 계정 아이디/비밀번호 입력 → **로그인 확인**
2. 텔레그램 봇 토큰 + 채팅 ID 입력 → **테스트 메시지 발송**
3. 알림 규칙 설정 (주기/시간, 근무시간 적용) → **설정 저장**

### 메일 알림
1. **메일 알림** 메뉴 → POP3 서버/계정/비밀번호 입력 → **연결 확인 및 저장**
2. 텔레그램 봇 토큰 + 채팅 ID 입력 → **테스트 메시지 발송**
3. 알림 스케줄 설정 → **설정 저장**

### 텔레그램 봇 생성 방법
1. 텔레그램에서 **@BotFather** 검색하여 대화 시작
2. `/newbot` 입력
3. 봇 이름 입력 (예: `D-Mate 알림`)
4. 봇 사용자명 입력 (예: `dmate_notify_bot`)
5. 발급된 **토큰**을 복사하여 D-Mate에 입력
6. 생성된 봇에게 아무 메시지를 보낸 후 D-Mate에서 **채팅 조회** 클릭

> SWORK 알림과 메일 알림에 같은 봇을 사용하거나 별도 봇을 만들 수 있습니다.

---

## 앱 동작

- **시스템 트레이**: 앱은 트레이에서 실행됩니다
- **X 닫기**: 창만 숨김 (앱은 계속 실행)
- **트레이 → 종료**: 앱 완전 종료
- **자동 시작**: 시스템 설정에서 활성화 가능

---

## 제거

### macOS
```bash
# 앱 삭제
rm -rf /Applications/D-Mate.app

# 데이터 삭제 (선택)
rm -rf ~/Library/Application\ Support/d-mate

# Keychain 삭제 (선택)
security delete-generic-password -s "d-mate" -a "credentials"
```

### Windows
- **설정 → 앱 → D-Mate → 제거**
- 데이터 삭제: `%APPDATA%/d-mate` 폴더 삭제
