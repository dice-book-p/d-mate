<script>
  import Card from "../components/Card.svelte";
  import ConnBadge from "../components/ConnBadge.svelte";
  import Toggle from "../components/Toggle.svelte";
  import Tooltip from "../components/Tooltip.svelte";
  import { settings, showToast, alerts as alertStore, pageDirty } from "../lib/stores.js";
  import { showDialog } from "../lib/dialog.js";
  import { isDirty, snapshot } from "../lib/dirty.js";
  import { getSettings, saveSettings, testTelegram, lookupTelegramChats, disconnectService, verifyMailLogin } from "../lib/api.js";

  let s = $state({});
  let mailAlerts = $state([]);
  let original = $state({});
  let mailPass = $state("");
  let hasMailPass = $state(false);
  let tgToken = $state("");
  let tgChatId = $state("");
  let chatList = $state([]);
  let testing = $state(false);
  let saving = $state(false);
  let savingMail = $state(false);
  let lookingUp = $state(false);

  let editMail = $state(false);
  let editTg = $state(false);
  let mailSaved = $state(false);
 // 저장 버튼으로 실제 저장 완료된 상태

  const DIRTY_FIELDS = [
    "mail_notify_enabled", "mail_schedule_type", "mail_interval_min", "mail_times", "mail_use_work_hours",
  ];

  const dirty = $derived(isDirty(original, s, DIRTY_FIELDS));
  $effect(() => { pageDirty.set(dirty); });

  settings.subscribe((v) => {
    if (v && Object.keys(v).length) {
      s = { ...v };
      original = snapshot(v, DIRTY_FIELDS);
      hasMailPass = v.has_mail_password || false;
      tgToken = v.mail_tg_token || "";
      tgChatId = v.mail_tg_chat_id || "";
      // 서버+계정+비밀번호 모두 있으면 이미 저장된 상태
      mailSaved = !!(v.mail_server && v.mail_account && v.has_mail_password);
    }
  });

  alertStore.subscribe((a) => {
    mailAlerts = (a || []).filter(al => al.source?.startsWith("mail"));
  });

  $effect(() => {
    if (!hasMailPass || !s.mail_account) editMail = true;
    if (!tgToken || !tgChatId) editTg = true;
  });

  const hasMailError = $derived(mailAlerts.some(a => a.id === "mail_auth" || a.id === "mail_server"));
  const hasTgError = $derived(mailAlerts.some(a => a.id?.includes("tg")));
  const mailOk = $derived(mailSaved && !hasMailError);
  const tgOk = $derived(tgToken && tgChatId && !hasTgError);

  async function lookupChats() {
    if (!tgToken) { showToast("봇 토큰을 입력하세요.", "error"); return; }
    lookingUp = true;
    try {
      const r = await lookupTelegramChats(tgToken);
      if (r.error) showToast(r.error, "error");
      else chatList = r.chats || [];
    } catch (e) { showToast("조회 실패", "error"); }
    lookingUp = false;
  }

  function selectChat(chatId) { tgChatId = chatId; chatList = []; }

  async function testMailTg() {
    testing = true;
    try {
      await saveSettings({ mail_tg_token: tgToken, mail_tg_chat_id: tgChatId });
      const r = await testTelegram("mail", tgToken, tgChatId);
      if (r.ok) {
        const fresh = await getSettings();
        settings.set(fresh);
        editTg = false;
      }
      showToast(r.ok ? "테스트 메시지 발송 성공!" : r.message, r.ok ? "success" : "error");
    } catch (e) { showToast("테스트 실패", "error"); }
    testing = false;
  }

  function disconnMail() {
    showDialog({
      type: "danger",
      title: "메일 서버 해지",
      message: "서버 설정과 비밀번호가 삭제되고\n메일 알림이 비활성화됩니다.",
      confirmText: "해지",
      async onConfirm() {
        try {
          await disconnectService("mail");
          const fresh = await getSettings();
          settings.set(fresh);
          mailPass = ""; hasMailPass = false; mailSaved = false;
          editMail = true;
          showToast("메일 서버 연결이 해제되었습니다.", "info");
        } catch (e) { showToast("해제 실패", "error"); }
      },
    });
  }

  function disconnMailTg() {
    showDialog({
      type: "danger",
      title: "메일 텔레그램 봇 해지",
      message: "봇 토큰과 채팅 ID가 삭제됩니다.\n정말 연결을 해제하시겠습니까?",
      confirmText: "해지",
      async onConfirm() {
        try {
          await disconnectService("mail_tg");
          const fresh = await getSettings();
          settings.set(fresh);
          tgToken = ""; tgChatId = "";
          editTg = true;
          showToast("텔레그램 봇 연결이 해제되었습니다.", "info");
        } catch (e) { showToast("해제 실패", "error"); }
      },
    });
  }

  async function saveMail() {
    if (!s.mail_server || !s.mail_account) { showToast("서버와 계정을 입력하세요.", "error"); return; }
    const pw = mailPass || (hasMailPass ? "__EXISTING__" : "");
    if (!pw) { showToast("비밀번호를 입력하세요.", "error"); return; }

    savingMail = true;
    try {
      // 1) 연결 테스트 (기존 비밀번호면 Keychain에서 가져옴)
      const testPw = mailPass || undefined;
      if (testPw) {
        // 새 비밀번호가 입력된 경우 → 먼저 저장 후 테스트
        await saveSettings({ mail_server: s.mail_server, mail_port: s.mail_port, mail_account: s.mail_account, mail_password: testPw });
      }

      const r = await verifyMailLogin(s.mail_server, s.mail_port, s.mail_use_ssl === 1, s.mail_account, testPw || "");
      if (!r.ok) {
        showToast(r.message, "error");
        savingMail = false;
        return;
      }

      // 2) 연결 성공 → 설정 저장
      const data = { mail_server: s.mail_server, mail_port: s.mail_port, mail_account: s.mail_account };
      if (mailPass) data.mail_password = mailPass;
      await saveSettings(data);
      const fresh = await getSettings();
      settings.set(fresh);
      if (mailPass) { mailPass = ""; hasMailPass = true; }
      mailSaved = true;
      editMail = false;
      showToast("메일 서버 연결 확인 및 저장 완료", "success");
    } catch (e) { showToast("저장 실패: " + e, "error"); }
    savingMail = false;
  }

  async function saveTgOnly() {
    if (!tgToken || !tgChatId) { showToast("봇 토큰과 채팅 ID를 입력하세요.", "error"); return; }
    try {
      await saveSettings({ mail_tg_token: tgToken, mail_tg_chat_id: tgChatId });
      const fresh = await getSettings();
      settings.set(fresh);
      editTg = false;
      showToast("텔레그램 봇 설정이 저장되었습니다.", "success");
    } catch (e) { showToast("저장 실패", "error"); }
  }

  async function save() {
    saving = true;
    try {
      const data = {
        mail_notify_enabled: s.mail_notify_enabled,
        mail_server: s.mail_server,
        mail_port: s.mail_port,
        mail_use_ssl: s.mail_use_ssl,
        mail_account: s.mail_account,
        mail_schedule_type: s.mail_schedule_type,
        mail_interval_min: s.mail_interval_min,
        mail_times: s.mail_times,
        mail_use_work_hours: s.mail_use_work_hours,
        mail_tg_token: tgToken,
        mail_tg_chat_id: tgChatId,
      };
      if (mailPass) data.mail_password = mailPass;
      await saveSettings(data);
      const fresh = await getSettings();
      settings.set(fresh);
      if (mailPass) { mailPass = ""; editMail = false; }
      pageDirty.set(false);
      showToast("저장되었습니다.", "success");
    } catch (e) { showToast("저장 실패", "error"); }
    saving = false;
  }

  function maskToken(t) {
    if (!t || t.length < 10) return t;
    return t.slice(0, 6) + "..." + t.slice(-4);
  }
</script>

<div class="page">
  <h2 class="page-title">메일 알림 설정</h2>

  <div class="section-grid">
    <!-- ① 메일 서버 -->
    <Card title="① 메일 서버">
      <div class="card-status-row">
        <ConnBadge
          state={hasMailError ? (mailAlerts.find(a=>a.id==="mail_auth") ? "auth_error" : "server_error") : (hasMailPass && s.mail_account ? "connected" : "not_configured")}
          error={mailAlerts.find(a=>a.id?.startsWith("mail_") && !a.id?.includes("tg"))?.message || ""}
          account={s.mail_account}
        />
      </div>

      {#if mailOk && !editMail}
        <div class="connected-info mt-12">
          <div class="info-item">
            <span class="info-label">서버</span>
            <strong>{s.mail_server}:{s.mail_port}</strong>
          </div>
          <div class="info-item">
            <span class="info-label">계정</span>
            <strong>{s.mail_account}</strong>
          </div>
          <div class="btn-row">
            <button class="btn btn-ghost-sm" onclick={() => { editMail = true; }}>재설정</button>
            <button class="btn btn-ghost-sm danger-text" onclick={disconnMail}>해지</button>
          </div>
        </div>
      {:else}
        <div class="form-group mt-12">
          <label for="mail-server">POP3 서버</label>
          <input id="mail-server" type="text" bind:value={s.mail_server} placeholder="webmail.daonplace.com" />
        </div>
        <div class="form-row">
          <div class="form-group flex-1">
            <label for="mail-port">포트</label>
            <input id="mail-port" type="number" bind:value={s.mail_port} placeholder="110" />
          </div>
        </div>
        <div class="form-group">
          <label for="mail-account">이메일 계정</label>
          <input id="mail-account" type="text" bind:value={s.mail_account} placeholder="user@daonplace.com" />
        </div>
        <div class="form-group">
          <label for="mail-pw">비밀번호 {hasMailPass ? "(설정됨)" : ""}</label>
          <input id="mail-pw" type="password" bind:value={mailPass} placeholder={hasMailPass ? "변경 시 입력" : "이메일 비밀번호"} />
          <small class="hint">OS Keychain에 안전하게 저장됩니다.</small>
        </div>
        <div class="btn-row">
          <button class="btn btn-primary" onclick={saveMail} disabled={savingMail}>
            {savingMail ? "연결 확인 중..." : "연결 확인 및 저장"}
          </button>
          {#if hasMailPass && s.mail_account}
            <button class="btn btn-ghost-sm" onclick={() => editMail = false}>취소</button>
          {/if}
        </div>
      {/if}
    </Card>

    <!-- ② 텔레그램 봇 -->
    <Card title="② 텔레그램 봇 (메일 알림용)">
      <div class="card-status-row">
        <ConnBadge
          state={hasTgError ? "auth_error" : (tgToken && tgChatId ? "connected" : "not_configured")}
          error={mailAlerts.find(a=>a.id?.includes("tg"))?.message || ""}
        />
      </div>

      {#if tgOk && !editTg}
        <div class="connected-info">
          <div class="info-item">
            <span class="info-label">봇 토큰</span>
            <strong>{maskToken(tgToken)}</strong>
          </div>
          <div class="info-item">
            <span class="info-label">채팅 ID</span>
            <strong>{tgChatId}</strong>
          </div>
          <div class="btn-row">
            <button class="btn btn-outline btn-sm" onclick={testMailTg} disabled={testing}>
              {testing ? "발송 중..." : "테스트 발송"}
            </button>
            <button class="btn btn-ghost-sm" onclick={() => editTg = true}>재설정</button>
            <button class="btn btn-ghost-sm danger-text" onclick={disconnMailTg}>해지</button>
          </div>
        </div>
      {:else}
        <div class="form-group">
          <label for="mtg-token">
            봇 토큰
            <Tooltip text="SWORK 알림 봇과 별도로 설정하거나, 동일한 봇 토큰을 공유할 수도 있습니다." />
          </label>
          <input id="mtg-token" type="text" bind:value={tgToken} placeholder="123456:ABC-..." />
        </div>
        <div class="form-group">
          <label for="mtg-chat">
            채팅 ID
            <Tooltip text="생성한 봇에게 아무 메시지를 보낸 뒤 '채팅 조회' 버튼을 누르면 채팅 ID가 자동 조회됩니다." />
          </label>
          <div class="input-with-btn">
            <input id="mtg-chat" type="text" bind:value={tgChatId} placeholder="채팅 ID" />
            <button class="btn btn-outline btn-sm" onclick={lookupChats} disabled={lookingUp}>
              {lookingUp ? "조회 중..." : "채팅 조회"}
            </button>
          </div>
        </div>
        {#if chatList.length > 0}
          <div class="chat-list">
            {#each chatList as chat}
              <button class="chat-item" onclick={() => selectChat(chat.chat_id)}>
                <span class="chat-type">{chat.type}</span>
                <strong>{chat.title}</strong>
                <span class="chat-id">{chat.chat_id}</span>
              </button>
            {/each}
          </div>
        {/if}
        <div class="btn-row">
          <button class="btn btn-primary" onclick={testMailTg} disabled={testing}>
            {testing ? "발송 중..." : "테스트 메시지 발송"}
          </button>
          <button class="btn btn-outline btn-sm" onclick={saveTgOnly}>저장만</button>
          {#if tgToken && tgChatId}
            <button class="btn btn-ghost-sm" onclick={() => editTg = false}>취소</button>
          {/if}
        </div>
      {/if}

      <div class="preview-box mt-12">
        <strong>알림 미리보기</strong>
        <div class="preview-content">
          <p><b>📬 새로운 메일이 도착했어요!</b></p>
          <p>보낸 사람: 홍길동</p>
          <p>제목: [공지] 주간 업무 보고</p>
        </div>
      </div>
    </Card>
  </div>

  <!-- ③ 알림 스케줄 -->
  <Card title="③ 알림 스케줄">
    <div class="rules-section">
      <Toggle label="메일 알림 전체" checked={s.mail_notify_enabled === 1} onchange={(v) => s.mail_notify_enabled = v ? 1 : 0} />
    </div>
    <div class="rule-block mt-12">
      <div class="schedule-selector">
        <label class="radio-label">
          <input type="radio" name="mail_type" value="interval" checked={s.mail_schedule_type === "interval"} onchange={() => s.mail_schedule_type = "interval"} />
          주기 반복
        </label>
        <label class="radio-label">
          <input type="radio" name="mail_type" value="times" checked={s.mail_schedule_type === "times"} onchange={() => s.mail_schedule_type = "times"} />
          특정 시간
        </label>
      </div>
      {#if s.mail_schedule_type === "interval"}
        <div class="schedule-detail">
          <label for="mail-interval">확인 주기</label>
          <select id="mail-interval" bind:value={s.mail_interval_min}>
            <option value={1}>1분</option><option value={2}>2분</option><option value={5}>5분</option><option value={10}>10분</option>
          </select>
          <span class="schedule-hint">매 정시 기준 ({s.mail_interval_min}분 → :00, :{String(s.mail_interval_min).padStart(2,"0")}...)</span>
        </div>
      {:else}
        <div class="schedule-detail">
          <label for="mail-times">알림 시간 (쉼표 구분)</label>
          <input id="mail-times" type="text" bind:value={s.mail_times} placeholder="08:50,13:20,17:50" />
        </div>
      {/if}
      <div class="work-hours-toggle">
        <Toggle label="근무시간 내에서만 발송" checked={s.mail_use_work_hours === 1} onchange={(v) => s.mail_use_work_hours = v ? 1 : 0} />
      </div>
    </div>
  </Card>

  <div class="save-bar">
    <button class="btn btn-primary btn-lg" onclick={save} disabled={saving || !dirty}>
      {saving ? "저장 중..." : dirty ? "설정 저장" : "변경사항 없음"}
    </button>
  </div>
</div>

<style>
  .preview-box { background: #f9fafb; border: 1px solid var(--c-border); border-radius: var(--radius-sm); padding: 12px 14px; }
  .preview-box strong { font-size: 12px; color: var(--c-text-secondary); margin-bottom: 8px; display: block; }
  .preview-content { font-size: 13px; line-height: 1.6; color: var(--c-text); }
  .preview-content b { color: var(--c-primary); }
</style>
