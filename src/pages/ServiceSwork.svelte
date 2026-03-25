<script>
  import Card from "../components/Card.svelte";
  import ConnBadge from "../components/ConnBadge.svelte";
  import Toggle from "../components/Toggle.svelte";
  import Tooltip from "../components/Tooltip.svelte";
  import { settings, showToast, alerts as alertStore, pageDirty } from "../lib/stores.js";
  import { showDialog } from "../lib/dialog.js";
  import { isDirty, snapshot } from "../lib/dirty.js";
  import {
    getSettings, saveSettings, verifySworkLogin, testTelegram, lookupTelegramChats, disconnectService,
  } from "../lib/api.js";

  let s = $state({});
  let sworkAlerts = $state([]);
  let sworkUser = $state("");
  let sworkPass = $state("");
  let hasSworkPass = $state(false);
  let tgToken = $state("");
  let tgChatId = $state("");
  let chatList = $state([]);
  let verifying = $state(false);
  let testing = $state(false);
  let saving = $state(false);
  let lookingUp = $state(false);

  let editSwork = $state(false);
  let editTg = $state(false);
  let original = $state({});

  const DIRTY_FIELDS = [
    "task_notify_enabled",
    "rule1_enabled", "rule1_schedule_type", "rule1_interval_min", "rule1_times", "rule1_use_work_hours",
    "rule2_enabled", "rule2_schedule_type", "rule2_interval_min", "rule2_times", "rule2_use_work_hours",
  ];

  const dirty = $derived(isDirty(original, s, DIRTY_FIELDS));
  $effect(() => { pageDirty.set(dirty); });

  settings.subscribe((v) => {
    if (v && Object.keys(v).length) {
      s = { ...v };
      original = snapshot(v, DIRTY_FIELDS);
      sworkUser = v.swork_username || "";
      hasSworkPass = v.has_swork_password || false;
      tgToken = v.swork_tg_token || "";
      tgChatId = v.swork_tg_chat_id || "";
    }
  });

  alertStore.subscribe((a) => {
    sworkAlerts = (a || []).filter(al => al.source?.startsWith("swork"));
  });

  $effect(() => {
    if (!hasSworkPass) editSwork = true;
    if (!tgToken || !tgChatId) editTg = true;
  });

  const hasSworkError = $derived(sworkAlerts.some(a => a.id === "swork_auth" || a.id === "swork_server"));
  const hasTgError = $derived(sworkAlerts.some(a => a.id?.includes("tg")));
  const sworkOk = $derived(hasSworkPass && !hasSworkError);
  const tgOk = $derived(tgToken && tgChatId && !hasTgError);

  async function verifySwork() {
    if (!sworkUser || !sworkPass) { showToast("아이디와 비밀번호를 입력하세요.", "error"); return; }
    verifying = true;
    try {
      const r = await verifySworkLogin(sworkUser, sworkPass);
      if (r.ok) {
        showToast("swork 로그인 성공!", "success");
        await saveSettings({ swork_username: sworkUser, swork_password: sworkPass });
        const fresh = await getSettings();
        settings.set(fresh);
        hasSworkPass = true;
        sworkPass = "";
        editSwork = false;
      } else {
        showToast(r.message, "error");
      }
    } catch (e) { showToast("연결 오류", "error"); }
    verifying = false;
  }

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

  async function testSworkTg() {
    testing = true;
    try {
      await saveSettings({ swork_tg_token: tgToken, swork_tg_chat_id: tgChatId });
      const r = await testTelegram("swork", tgToken, tgChatId);
      if (r.ok) {
        const fresh = await getSettings();
        settings.set(fresh);
        editTg = false;
      }
      showToast(r.ok ? "테스트 메시지 발송 성공!" : r.message, r.ok ? "success" : "error");
    } catch (e) { showToast("테스트 실패", "error"); }
    testing = false;
  }

  function disconnSwork() {
    showDialog({
      type: "danger",
      title: "swork 계정 해지",
      message: "저장된 아이디와 비밀번호가 삭제됩니다.\n정말 연결을 해제하시겠습니까?",
      confirmText: "해지",
      async onConfirm() {
        try {
          await disconnectService("swork");
          const fresh = await getSettings();
          settings.set(fresh);
          sworkUser = ""; sworkPass = ""; hasSworkPass = false;
          editSwork = true;
          showToast("swork 계정 연결이 해제되었습니다.", "info");
        } catch (e) { showToast("해제 실패", "error"); }
      },
    });
  }

  function disconnSworkTg() {
    showDialog({
      type: "danger",
      title: "SWORK 텔레그램 봇 해지",
      message: "봇 토큰과 채팅 ID가 삭제됩니다.\n정말 연결을 해제하시겠습니까?",
      confirmText: "해지",
      async onConfirm() {
        try {
          await disconnectService("swork_tg");
          const fresh = await getSettings();
          settings.set(fresh);
          tgToken = ""; tgChatId = "";
          editTg = true;
          showToast("텔레그램 봇 연결이 해제되었습니다.", "info");
        } catch (e) { showToast("해제 실패", "error"); }
      },
    });
  }

  async function saveTgOnly() {
    if (!tgToken || !tgChatId) { showToast("봇 토큰과 채팅 ID를 입력하세요.", "error"); return; }
    try {
      await saveSettings({ swork_tg_token: tgToken, swork_tg_chat_id: tgChatId });
      const fresh = await getSettings();
      settings.set(fresh);
      editTg = false;
      showToast("텔레그램 봇 설정이 저장되었습니다.", "success");
    } catch (e) { showToast("저장 실패", "error"); }
  }

  async function save() {
    saving = true;
    try {
      await saveSettings({
        task_notify_enabled: s.task_notify_enabled,
        rule1_enabled: s.rule1_enabled,
        rule1_schedule_type: s.rule1_schedule_type,
        rule1_interval_min: s.rule1_interval_min,
        rule1_times: s.rule1_times,
        rule1_use_work_hours: s.rule1_use_work_hours,
        rule2_enabled: s.rule2_enabled,
        rule2_schedule_type: s.rule2_schedule_type,
        rule2_interval_min: s.rule2_interval_min,
        rule2_times: s.rule2_times,
        rule2_use_work_hours: s.rule2_use_work_hours,
        swork_tg_token: tgToken,
        swork_tg_chat_id: tgChatId,
      });
      const fresh = await getSettings();
      settings.set(fresh);
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
  <h2 class="page-title">SWORK 알림 설정</h2>

  <div class="section-grid">
    <!-- ① swork 계정 -->
    <Card title="① swork 계정">
      <div class="card-status-row">
        <ConnBadge
          state={hasSworkError ? (sworkAlerts.find(a=>a.id==="swork_auth") ? "auth_error" : "server_error") : hasSworkPass ? "connected" : "not_configured"}
          error={sworkAlerts.find(a=>a.id?.startsWith("swork_"))?.message || ""}
          account={sworkUser}
        />
      </div>

      {#if sworkOk && !editSwork}
        <div class="connected-info">
          <div class="info-item">
            <span class="info-label">계정</span>
            <strong>{sworkUser}</strong>
          </div>
          <div class="btn-row">
            <button class="btn btn-ghost-sm" onclick={() => editSwork = true}>재설정</button>
            <button class="btn btn-ghost-sm danger-text" onclick={disconnSwork}>해지</button>
          </div>
        </div>
      {:else}
        <div class="form-group">
          <label for="swork-id">아이디</label>
          <input id="swork-id" type="text" bind:value={sworkUser} placeholder="swork 아이디" />
        </div>
        <div class="form-group">
          <label for="swork-pw">비밀번호 {hasSworkPass ? "(설정됨)" : ""}</label>
          <input id="swork-pw" type="password" bind:value={sworkPass} placeholder={hasSworkPass ? "변경 시 입력" : "비밀번호"} />
        </div>
        <div class="btn-row">
          <button class="btn btn-primary" onclick={verifySwork} disabled={verifying}>
            {verifying ? "확인 중..." : "로그인 확인"}
          </button>
          {#if hasSworkPass}
            <button class="btn btn-ghost-sm" onclick={() => editSwork = false}>취소</button>
          {/if}
        </div>
      {/if}
    </Card>

    <!-- ② 텔레그램 봇 -->
    <Card title="② 텔레그램 봇 (SWORK 알림용)">
      <div class="card-status-row">
        <ConnBadge
          state={hasTgError ? "auth_error" : (tgToken && tgChatId ? "connected" : "not_configured")}
          error={sworkAlerts.find(a=>a.id?.includes("tg"))?.message || ""}
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
            <button class="btn btn-outline btn-sm" onclick={testSworkTg} disabled={testing}>
              {testing ? "발송 중..." : "테스트 발송"}
            </button>
            <button class="btn btn-ghost-sm" onclick={() => editTg = true}>재설정</button>
            <button class="btn btn-ghost-sm danger-text" onclick={disconnSworkTg}>해지</button>
          </div>
        </div>
      {:else}
        <div class="form-group">
          <label for="stg-token">
            봇 토큰
            <Tooltip text="텔레그램에서 @BotFather와 대화하여 /newbot으로 봇을 생성한 뒤, 발급된 토큰을 여기에 입력합니다." />
          </label>
          <input id="stg-token" type="text" bind:value={tgToken} placeholder="123456:ABC-..." />
        </div>
        <div class="form-group">
          <label for="stg-chat">
            채팅 ID
            <Tooltip text="생성한 봇에게 아무 메시지를 보낸 뒤 '채팅 조회' 버튼을 누르면 채팅 ID가 자동 조회됩니다." />
          </label>
          <div class="input-with-btn">
            <input id="stg-chat" type="text" bind:value={tgChatId} placeholder="채팅 ID" />
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
          <button class="btn btn-primary" onclick={testSworkTg} disabled={testing}>
            {testing ? "발송 중..." : "테스트 메시지 발송"}
          </button>
          <button class="btn btn-outline btn-sm" onclick={saveTgOnly}>저장만</button>
          {#if tgToken && tgChatId}
            <button class="btn btn-ghost-sm" onclick={() => editTg = false}>취소</button>
          {/if}
        </div>
      {/if}
    </Card>
  </div>

  <!-- ③ 알림 규칙 -->
  <Card title="③ 알림 규칙">
    <div class="rules-section">
      <Toggle label="SWORK 알림 전체" checked={s.task_notify_enabled === 1} onchange={(v) => s.task_notify_enabled = v ? 1 : 0} />

      <div class="rule-block">
        <div class="rule-header">
          <Toggle label="승인/검수 요청 알림" checked={s.rule1_enabled === 1} onchange={(v) => s.rule1_enabled = v ? 1 : 0} />
          <Tooltip text="나에게 할당된 승인요청/검수요청 업무를 체크하여 텔레그램으로 알려줍니다." />
        </div>
        <div class="schedule-selector">
          <label class="radio-label">
            <input type="radio" name="rule1_type" value="interval" checked={s.rule1_schedule_type === "interval"} onchange={() => s.rule1_schedule_type = "interval"} />
            주기 반복
          </label>
          <label class="radio-label">
            <input type="radio" name="rule1_type" value="times" checked={s.rule1_schedule_type === "times"} onchange={() => s.rule1_schedule_type = "times"} />
            특정 시간
          </label>
        </div>
        {#if s.rule1_schedule_type === "interval"}
          <div class="schedule-detail">
            <label for="r1-interval">확인 주기</label>
            <select id="r1-interval" bind:value={s.rule1_interval_min}>
              <option value={2}>2분</option><option value={5}>5분</option><option value={10}>10분</option><option value={15}>15분</option><option value={30}>30분</option>
            </select>
            <span class="schedule-hint">매 정시 기준 ({s.rule1_interval_min}분 → :00, :{String(s.rule1_interval_min).padStart(2,"0")}, :{String(s.rule1_interval_min*2).padStart(2,"0")}...)</span>
          </div>
        {:else}
          <div class="schedule-detail">
            <label for="r1-times">알림 시간 (쉼표 구분)</label>
            <input id="r1-times" type="text" bind:value={s.rule1_times} placeholder="08:50,13:20,17:50" />
          </div>
        {/if}
        <div class="work-hours-toggle">
          <Toggle label="근무시간 내에서만 발송" checked={s.rule1_use_work_hours === 1} onchange={(v) => s.rule1_use_work_hours = v ? 1 : 0} />
        </div>
      </div>

      <div class="rule-block">
        <div class="rule-header">
          <Toggle label="지연 업무 알림" checked={s.rule2_enabled === 1} onchange={(v) => s.rule2_enabled = v ? 1 : 0} />
          <Tooltip text="마감일이 지난 업무를 텔레그램으로 알려줍니다." />
        </div>
        <div class="schedule-selector">
          <label class="radio-label">
            <input type="radio" name="rule2_type" value="interval" checked={s.rule2_schedule_type === "interval"} onchange={() => s.rule2_schedule_type = "interval"} />
            주기 반복
          </label>
          <label class="radio-label">
            <input type="radio" name="rule2_type" value="times" checked={s.rule2_schedule_type === "times"} onchange={() => s.rule2_schedule_type = "times"} />
            특정 시간
          </label>
        </div>
        {#if s.rule2_schedule_type === "interval"}
          <div class="schedule-detail">
            <label for="r2-interval">확인 주기</label>
            <select id="r2-interval" bind:value={s.rule2_interval_min}>
              <option value={2}>2분</option><option value={5}>5분</option><option value={10}>10분</option><option value={15}>15분</option><option value={30}>30분</option>
            </select>
            <span class="schedule-hint">매 정시 기준 ({s.rule2_interval_min}분 → :00, :{String(s.rule2_interval_min).padStart(2,"0")}, :{String(s.rule2_interval_min*2).padStart(2,"0")}...)</span>
          </div>
        {:else}
          <div class="schedule-detail">
            <label for="r2-times">알림 시간 (쉼표 구분)</label>
            <input id="r2-times" type="text" bind:value={s.rule2_times} placeholder="08:50,13:20,17:50" />
          </div>
        {/if}
        <div class="work-hours-toggle">
          <Toggle label="근무시간 내에서만 발송" checked={s.rule2_use_work_hours === 1} onchange={(v) => s.rule2_use_work_hours = v ? 1 : 0} />
        </div>
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
</style>
