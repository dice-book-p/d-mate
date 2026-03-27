<script>
  import { onMount, onDestroy } from "svelte";
  import Card from "../components/Card.svelte";
  import ConnBadge from "../components/ConnBadge.svelte";
  import Tooltip from "../components/Tooltip.svelte";
  import { settings, showToast, alerts as alertStore, pageDirty, deskState } from "../lib/stores.js";
  import { showDialog } from "../lib/dialog.js";
  import {
    getSettings, saveSettings, verifySworkLogin, verifyMailLogin,
    testTelegram, lookupTelegramChats, disconnectService, deskJoin, deskHealth, deskDisconnect,
    deskRequestJoin, deskCheckJoinStatus, deskCancelJoinRequest,
    getHostname,
  } from "../lib/api.js";

  let activeTab = $state("swork");
  let s = $state({});
  let allAlerts = $state([]);

  // ── Desk 상태 ──
  let deskServerUrl = $state("http://192.168.204.53:29180");
  let deskName = $state("");
  let deskDeviceName = $state("");
  let deskConnected = $state(false);
  let deskReachable = $state(false);
  let joiningDesk = $state(false);
  let deskChecked = $state(false);
  let joinPending = $state(false);
  let joinRequestId = $state(null);
  let pollInterval = null;

  // ── SWORK 상태 ──
  let sworkUser = $state("");
  let sworkPass = $state("");
  let hasSworkPass = $state(false);
  let sworkTgToken = $state("");
  let sworkTgChatId = $state("");
  let sworkChatList = $state([]);
  let editSwork = $state(false);
  let editSworkTg = $state(false);
  let verifying = $state(false);
  let testingSworkTg = $state(false);
  let lookingUpSwork = $state(false);

  // ── 메일 상태 ──
  let mailPass = $state("");
  let hasMailPass = $state(false);
  let mailTgToken = $state("");
  let mailTgChatId = $state("");
  let mailChatList = $state([]);
  let editMail = $state(false);
  let editMailTg = $state(false);
  let savingMail = $state(false);
  let testingMailTg = $state(false);
  let lookingUpMail = $state(false);
  let mailSaved = $state(false);

  onMount(async () => {
    let ds;
    const unsub = deskState.subscribe(v => ds = v);
    unsub();

    if (ds.checked) {
      // store에서 복원 (페이지 전환 시 재조회 안 함)
      deskConnected = ds.connected;
      deskReachable = ds.reachable;
      deskServerUrl = ds.serverUrl || "http://192.168.204.53:29180";
      deskChecked = true;
      if (ds.joinPending) {
        joinPending = true;
        joinRequestId = ds.joinRequestId;
        startJoinPolling();
      }
    } else {
      // 최초 체크
      try {
        const r = await deskHealth();
        deskReachable = r.reachable;
        deskConnected = r.connected;
        deskChecked = true;
        if (r.connected && r.server_url) {
          deskServerUrl = r.server_url;
        }
        deskState.set({ connected: r.connected, reachable: r.reachable, serverUrl: r.server_url || "", joinPending: false, joinRequestId: null, checked: true });
      } catch (_) {
        deskChecked = true;
        deskState.set({ connected: false, reachable: false, serverUrl: "", joinPending: false, joinRequestId: null, checked: true });
      }

      // 기존 pending 요청이 있으면 폴링 재개
      if (!deskConnected) {
        try {
          const status = await deskCheckJoinStatus();
          if (status?.data?.status === "pending") {
            joinPending = true;
            deskState.update(s => ({ ...s, joinPending: true }));
            startJoinPolling();
          } else if (status?.data?.status === "approved") {
            deskConnected = true;
            deskReachable = true;
            deskState.set({ connected: true, reachable: true, serverUrl: deskServerUrl, joinPending: false, joinRequestId: null, checked: true });
          }
        } catch (_) { /* 요청 정보 없으면 무시 */ }
      }
    }

    // Desk 디바이스명 기본값: PC hostname
    if (!deskDeviceName) {
      try {
        deskDeviceName = await getHostname();
      } catch (_) { /* ignore */ }
    }
  });

  onDestroy(() => {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  });

  settings.subscribe((v) => {
    if (v && Object.keys(v).length) {
      s = { ...v };
      sworkUser = v.swork_username || "";
      hasSworkPass = v.has_swork_password || false;
      sworkTgToken = v.swork_tg_token || "";
      sworkTgChatId = v.swork_tg_chat_id || "";
      hasMailPass = v.has_mail_password || false;
      mailTgToken = v.mail_tg_token || "";
      mailTgChatId = v.mail_tg_chat_id || "";
      mailSaved = !!(v.mail_server && v.mail_account && v.has_mail_password);
    }
  });

  alertStore.subscribe((a) => { allAlerts = a || []; });

  $effect(() => {
    if (!hasSworkPass) editSwork = true;
    if (!sworkTgToken || !sworkTgChatId) editSworkTg = true;
    if (!hasMailPass || !s.mail_account) editMail = true;
    if (!mailTgToken || !mailTgChatId) editMailTg = true;
  });

  const sworkAlerts = $derived(allAlerts.filter(a => a.source?.startsWith("swork")));
  const mailAlerts = $derived(allAlerts.filter(a => a.source?.startsWith("mail")));
  const hasSworkError = $derived(sworkAlerts.some(a => a.id === "swork_auth" || a.id === "swork_server"));
  const hasSworkTgError = $derived(sworkAlerts.some(a => a.id?.includes("tg")));
  const sworkOk = $derived(hasSworkPass && !hasSworkError);
  const sworkTgOk = $derived(sworkTgToken && sworkTgChatId && !hasSworkTgError);
  const hasMailError = $derived(mailAlerts.some(a => a.id === "mail_auth" || a.id === "mail_server"));
  const hasMailTgError = $derived(mailAlerts.some(a => a.id?.includes("tg")));
  const mailOk = $derived(mailSaved && !hasMailError);
  const mailTgOk = $derived(mailTgToken && mailTgChatId && !hasMailTgError);

  const sworkConnected = $derived(sworkOk && sworkTgOk);
  const mailConnected = $derived(mailOk && mailTgOk);

  function maskToken(t) {
    if (!t || t.length < 10) return t;
    return t.slice(0, 6) + "..." + t.slice(-4);
  }

  async function refreshSettings() {
    const fresh = await getSettings();
    settings.set(fresh);
  }

  // ── SWORK 함수 ──
  async function verifySwork() {
    if (!sworkUser || !sworkPass) { showToast("아이디와 비밀번호를 입력하세요.", "error"); return; }
    verifying = true;
    try {
      const r = await verifySworkLogin(sworkUser, sworkPass);
      if (r.ok) {
        showToast("swork 로그인 성공!", "success");
        await saveSettings({ swork_username: sworkUser, swork_password: sworkPass });
        await refreshSettings();
        hasSworkPass = true; sworkPass = ""; editSwork = false;
      } else { showToast(r.message, "error"); }
    } catch (e) { showToast("연결 오류", "error"); }
    verifying = false;
  }

  async function lookupSworkChats() {
    if (!sworkTgToken) { showToast("봇 토큰을 입력하세요.", "error"); return; }
    lookingUpSwork = true;
    try {
      const r = await lookupTelegramChats(sworkTgToken);
      if (r.error) showToast(r.error, "error");
      else sworkChatList = r.chats || [];
    } catch (e) { showToast("조회 실패", "error"); }
    lookingUpSwork = false;
  }

  async function testSworkTg() {
    testingSworkTg = true;
    try {
      await saveSettings({ swork_tg_token: sworkTgToken, swork_tg_chat_id: sworkTgChatId });
      const r = await testTelegram("swork", sworkTgToken, sworkTgChatId);
      if (r.ok) { await refreshSettings(); editSworkTg = false; }
      showToast(r.ok ? "테스트 메시지 발송 성공!" : r.message, r.ok ? "success" : "error");
    } catch (e) { showToast("테스트 실패", "error"); }
    testingSworkTg = false;
  }

  async function saveSworkTgOnly() {
    if (!sworkTgToken || !sworkTgChatId) { showToast("봇 토큰과 채팅 ID를 입력하세요.", "error"); return; }
    try {
      await saveSettings({ swork_tg_token: sworkTgToken, swork_tg_chat_id: sworkTgChatId });
      await refreshSettings();
      editSworkTg = false;
      showToast("텔레그램 봇 설정이 저장되었습니다.", "success");
    } catch (e) { showToast("저장 실패", "error"); }
  }

  function disconnSwork() {
    showDialog({ type: "danger", title: "swork 계정 해지", message: "저장된 아이디와 비밀번호가 삭제됩니다.\n정말 연결을 해제하시겠습니까?", confirmText: "해지",
      async onConfirm() {
        try { await disconnectService("swork"); await refreshSettings(); sworkUser = ""; sworkPass = ""; hasSworkPass = false; editSwork = true; showToast("swork 계정 연결이 해제되었습니다.", "info"); }
        catch (e) { showToast("해제 실패", "error"); }
      },
    });
  }

  function disconnSworkTg() {
    showDialog({ type: "danger", title: "SWORK 텔레그램 봇 해지", message: "봇 토큰과 채팅 ID가 삭제됩니다.", confirmText: "해지",
      async onConfirm() {
        try { await disconnectService("swork_tg"); await refreshSettings(); sworkTgToken = ""; sworkTgChatId = ""; editSworkTg = true; showToast("텔레그램 봇 연결이 해제되었습니다.", "info"); }
        catch (e) { showToast("해제 실패", "error"); }
      },
    });
  }

  // ── 메일 함수 ──
  async function saveMail() {
    if (!s.mail_server || !s.mail_account) { showToast("서버와 계정을 입력하세요.", "error"); return; }
    const pw = mailPass || (hasMailPass ? "__EXISTING__" : "");
    if (!pw) { showToast("비밀번호를 입력하세요.", "error"); return; }
    savingMail = true;
    try {
      const testPw = mailPass || undefined;
      if (testPw) { await saveSettings({ mail_server: s.mail_server, mail_port: s.mail_port, mail_account: s.mail_account, mail_password: testPw }); }
      const r = await verifyMailLogin(s.mail_server, s.mail_port, s.mail_use_ssl === 1, s.mail_account, testPw || "");
      if (!r.ok) { showToast(r.message, "error"); savingMail = false; return; }
      const data = { mail_server: s.mail_server, mail_port: s.mail_port, mail_account: s.mail_account };
      if (mailPass) data.mail_password = mailPass;
      await saveSettings(data);
      await refreshSettings();
      if (mailPass) { mailPass = ""; hasMailPass = true; }
      mailSaved = true; editMail = false;
      showToast("메일 서버 연결 확인 및 저장 완료", "success");
    } catch (e) { showToast("저장 실패: " + e, "error"); }
    savingMail = false;
  }

  async function lookupMailChats() {
    if (!mailTgToken) { showToast("봇 토큰을 입력하세요.", "error"); return; }
    lookingUpMail = true;
    try {
      const r = await lookupTelegramChats(mailTgToken);
      if (r.error) showToast(r.error, "error");
      else mailChatList = r.chats || [];
    } catch (e) { showToast("조회 실패", "error"); }
    lookingUpMail = false;
  }

  async function testMailTg() {
    testingMailTg = true;
    try {
      await saveSettings({ mail_tg_token: mailTgToken, mail_tg_chat_id: mailTgChatId });
      const r = await testTelegram("mail", mailTgToken, mailTgChatId);
      if (r.ok) { await refreshSettings(); editMailTg = false; }
      showToast(r.ok ? "테스트 메시지 발송 성공!" : r.message, r.ok ? "success" : "error");
    } catch (e) { showToast("테스트 실패", "error"); }
    testingMailTg = false;
  }

  async function saveMailTgOnly() {
    if (!mailTgToken || !mailTgChatId) { showToast("봇 토큰과 채팅 ID를 입력하세요.", "error"); return; }
    try {
      await saveSettings({ mail_tg_token: mailTgToken, mail_tg_chat_id: mailTgChatId });
      await refreshSettings();
      editMailTg = false;
      showToast("텔레그램 봇 설정이 저장되었습니다.", "success");
    } catch (e) { showToast("저장 실패", "error"); }
  }

  function disconnMail() {
    showDialog({ type: "danger", title: "메일 서버 해지", message: "서버 설정과 비밀번호가 삭제되고\n메일 알림이 비활성화됩니다.", confirmText: "해지",
      async onConfirm() {
        try { await disconnectService("mail"); await refreshSettings(); mailPass = ""; hasMailPass = false; mailSaved = false; editMail = true; showToast("메일 서버 연결이 해제되었습니다.", "info"); }
        catch (e) { showToast("해제 실패", "error"); }
      },
    });
  }

  function disconnMailTg() {
    showDialog({ type: "danger", title: "메일 텔레그램 봇 해지", message: "봇 토큰과 채팅 ID가 삭제됩니다.", confirmText: "해지",
      async onConfirm() {
        try { await disconnectService("mail_tg"); await refreshSettings(); mailTgToken = ""; mailTgChatId = ""; editMailTg = true; showToast("텔레그램 봇 연결이 해제되었습니다.", "info"); }
        catch (e) { showToast("해제 실패", "error"); }
      },
    });
  }

  // ── Desk 함수 ──
  async function requestJoin() {
    if (!deskServerUrl || !deskName || !deskDeviceName) {
      showToast("모든 항목을 입력하세요.", "error"); return;
    }
    joiningDesk = true;
    try {
      const res = await deskRequestJoin(deskServerUrl, deskName, deskDeviceName);
      if (res?.ok) {
        joinPending = true;
        joinRequestId = res.data?.request_id;
        deskState.update(s => ({ ...s, joinPending: true, joinRequestId: res.data?.request_id }));
        showToast("참여 요청이 접수되었습니다. 관리자 승인을 기다려주세요.", "info");
        startJoinPolling();
      } else {
        showToast(res?.message || "요청 실패", "error");
      }
    } catch (e) { showToast("요청 오류: " + e, "error"); }
    joiningDesk = false;
  }

  function startJoinPolling() {
    if (pollInterval) clearInterval(pollInterval);
    pollInterval = setInterval(async () => {
      try {
        const res = await deskCheckJoinStatus();
        if (res?.data?.status === "approved") {
          clearInterval(pollInterval);
          pollInterval = null;
          joinPending = false;
          deskConnected = true;
          deskReachable = true;
          if (res.data?.server_url) deskServerUrl = res.data.server_url;
          deskState.set({ connected: true, reachable: true, serverUrl: deskServerUrl, joinPending: false, joinRequestId: null, checked: true });
          showToast("승인되었습니다! Desk에 연결되었습니다.", "success");
        } else if (res?.data?.status === "rejected") {
          clearInterval(pollInterval);
          pollInterval = null;
          joinPending = false;
          deskState.update(s => ({ ...s, joinPending: false, joinRequestId: null }));
          showToast(`요청이 거부되었습니다${res.data.reason ? ": " + res.data.reason : ""}`, "error");
        }
      } catch (_) { /* 폴링 실패는 무시 — 다음 주기에 재시도 */ }
    }, 5000);
  }

  async function cancelJoinRequest() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
    try {
      await deskCancelJoinRequest();
      joinPending = false;
      joinRequestId = null;
      deskState.update(s => ({ ...s, joinPending: false, joinRequestId: null }));
      showToast("참여 요청이 취소되었습니다.", "info");
    } catch (e) { showToast("취소 실패", "error"); }
  }

  async function checkDeskHealth() {
    try {
      const r = await deskHealth();
      deskReachable = r.reachable;
      deskConnected = r.connected;
      deskState.set({ connected: r.connected, reachable: r.reachable, serverUrl: r.server_url || deskServerUrl, joinPending, joinRequestId, checked: true });
      showToast(r.reachable ? "서버 정상" : "서버 응답 없음", r.reachable ? "success" : "error");
    } catch (e) { showToast("확인 실패", "error"); }
  }
</script>

<div class="page">
  <h2 class="page-title">연결 관리</h2>

  <div class="tab-bar">
    <button class="tab" class:active={activeTab === "swork"} onclick={() => activeTab = "swork"}>
      SWORK
      <span class="tab-dot" class:green={sworkConnected} class:red={!sworkConnected}></span>
    </button>
    <button class="tab" class:active={activeTab === "mail"} onclick={() => activeTab = "mail"}>
      메일
      <span class="tab-dot" class:green={mailConnected} class:red={!mailConnected && (hasMailPass || s.mail_account)}></span>
    </button>
    <button class="tab" class:active={activeTab === "desk"} onclick={() => activeTab = "desk"}>
      Desk
      <span class="tab-dot" class:green={deskConnected && deskReachable} class:red={deskConnected && !deskReachable} class:yellow={joinPending} class:gray={!deskConnected && !joinPending}></span>
    </button>
  </div>

  <!-- SWORK 탭 -->
  {#if activeTab === "swork"}
    <div class="tab-content">
      <div class="section-grid">
        <Card title="swork 계정">
          <div class="card-status-row">
            <ConnBadge
              state={hasSworkError ? (sworkAlerts.find(a=>a.id==="swork_auth") ? "auth_error" : "server_error") : hasSworkPass ? "connected" : "not_configured"}
              error={sworkAlerts.find(a=>a.id?.startsWith("swork_"))?.message || ""}
              account={sworkUser}
            />
          </div>
          {#if sworkOk && !editSwork}
            <div class="connected-info">
              <div class="info-item"><span class="info-label">계정</span><strong>{sworkUser}</strong></div>
              <div class="btn-row">
                <button class="btn btn-ghost-sm" onclick={() => editSwork = true}>재설정</button>
                <button class="btn btn-ghost-sm danger-text" onclick={disconnSwork}>해지</button>
              </div>
            </div>
          {:else}
            <div class="form-group"><label for="swork-id">아이디</label><input id="swork-id" type="text" bind:value={sworkUser} placeholder="swork 아이디" /></div>
            <div class="form-group"><label for="swork-pw">비밀번호 {hasSworkPass ? "(설정됨)" : ""}</label><input id="swork-pw" type="password" bind:value={sworkPass} placeholder={hasSworkPass ? "변경 시 입력" : "비밀번호"} /></div>
            <div class="btn-row">
              <button class="btn btn-primary" onclick={verifySwork} disabled={verifying}>{verifying ? "확인 중..." : "로그인 확인"}</button>
              {#if hasSworkPass}<button class="btn btn-ghost-sm" onclick={() => editSwork = false}>취소</button>{/if}
            </div>
          {/if}
        </Card>

        <Card title="텔레그램 봇 (SWORK 알림용)">
          <div class="card-status-row">
            <ConnBadge state={hasSworkTgError ? "auth_error" : (sworkTgToken && sworkTgChatId ? "connected" : "not_configured")} error={sworkAlerts.find(a=>a.id?.includes("tg"))?.message || ""} />
          </div>
          {#if sworkTgOk && !editSworkTg}
            <div class="connected-info">
              <div class="info-item"><span class="info-label">봇 토큰</span><strong>{maskToken(sworkTgToken)}</strong></div>
              <div class="info-item"><span class="info-label">채팅 ID</span><strong>{sworkTgChatId}</strong></div>
              <div class="btn-row">
                <button class="btn btn-outline btn-sm" onclick={testSworkTg} disabled={testingSworkTg}>{testingSworkTg ? "발송 중..." : "테스트 발송"}</button>
                <button class="btn btn-ghost-sm" onclick={() => editSworkTg = true}>재설정</button>
                <button class="btn btn-ghost-sm danger-text" onclick={disconnSworkTg}>해지</button>
              </div>
            </div>
          {:else}
            <div class="form-group">
              <label for="stg-token">봇 토큰 <Tooltip text="텔레그램에서 @BotFather와 대화하여 /newbot으로 봇을 생성한 뒤, 발급된 토큰을 여기에 입력합니다." /></label>
              <input id="stg-token" type="text" bind:value={sworkTgToken} placeholder="123456:ABC-..." />
            </div>
            <div class="form-group">
              <label for="stg-chat">채팅 ID <Tooltip text="생성한 봇에게 아무 메시지를 보낸 뒤 '채팅 조회' 버튼을 누르면 채팅 ID가 자동 조회됩니다." /></label>
              <div class="input-with-btn">
                <input id="stg-chat" type="text" bind:value={sworkTgChatId} placeholder="채팅 ID" />
                <button class="btn btn-outline btn-sm" onclick={lookupSworkChats} disabled={lookingUpSwork}>{lookingUpSwork ? "조회 중..." : "채팅 조회"}</button>
              </div>
            </div>
            {#if sworkChatList.length > 0}
              <div class="chat-list">{#each sworkChatList as chat}<button class="chat-item" onclick={() => { sworkTgChatId = chat.chat_id; sworkChatList = []; }}><span class="chat-type">{chat.type}</span><strong>{chat.title}</strong><span class="chat-id">{chat.chat_id}</span></button>{/each}</div>
            {/if}
            <div class="btn-row">
              <button class="btn btn-primary" onclick={testSworkTg} disabled={testingSworkTg}>{testingSworkTg ? "발송 중..." : "테스트 메시지 발송"}</button>
              <button class="btn btn-outline btn-sm" onclick={saveSworkTgOnly}>저장만</button>
              {#if sworkTgToken && sworkTgChatId}<button class="btn btn-ghost-sm" onclick={() => editSworkTg = false}>취소</button>{/if}
            </div>
          {/if}
        </Card>
      </div>
    </div>
  {/if}

  <!-- 메일 탭 -->
  {#if activeTab === "mail"}
    <div class="tab-content">
      <div class="section-grid">
        <Card title="메일 서버">
          <div class="card-status-row">
            <ConnBadge state={hasMailError ? (mailAlerts.find(a=>a.id==="mail_auth") ? "auth_error" : "server_error") : (hasMailPass && s.mail_account ? "connected" : "not_configured")} error={mailAlerts.find(a=>a.id?.startsWith("mail_") && !a.id?.includes("tg"))?.message || ""} account={s.mail_account} />
          </div>
          {#if mailOk && !editMail}
            <div class="connected-info mt-12">
              <div class="info-item"><span class="info-label">서버</span><strong>{s.mail_server}:{s.mail_port}</strong></div>
              <div class="info-item"><span class="info-label">계정</span><strong>{s.mail_account}</strong></div>
              <div class="btn-row">
                <button class="btn btn-ghost-sm" onclick={() => editMail = true}>재설정</button>
                <button class="btn btn-ghost-sm danger-text" onclick={disconnMail}>해지</button>
              </div>
            </div>
          {:else}
            <div class="form-group mt-12"><label for="mail-server">POP3 서버</label><input id="mail-server" type="text" bind:value={s.mail_server} placeholder="webmail.daonplace.com" /></div>
            <div class="form-row">
              <div class="form-group flex-1"><label for="mail-port">포트</label><input id="mail-port" type="number" bind:value={s.mail_port} placeholder="110" /></div>
            </div>
            <div class="form-group"><label for="mail-account">이메일 계정</label><input id="mail-account" type="text" bind:value={s.mail_account} placeholder="user@daonplace.com" /></div>
            <div class="form-group">
              <label for="mail-pw">비밀번호 {hasMailPass ? "(설정됨)" : ""}</label>
              <input id="mail-pw" type="password" bind:value={mailPass} placeholder={hasMailPass ? "변경 시 입력" : "이메일 비밀번호"} />
              <small class="hint">OS Keychain에 안전하게 저장됩니다.</small>
            </div>
            <div class="btn-row">
              <button class="btn btn-primary" onclick={saveMail} disabled={savingMail}>{savingMail ? "연결 확인 중..." : "연결 확인 및 저장"}</button>
              {#if hasMailPass && s.mail_account}<button class="btn btn-ghost-sm" onclick={() => editMail = false}>취소</button>{/if}
            </div>
          {/if}
        </Card>

        <Card title="텔레그램 봇 (메일 알림용)">
          <div class="card-status-row">
            <ConnBadge state={hasMailTgError ? "auth_error" : (mailTgToken && mailTgChatId ? "connected" : "not_configured")} error={mailAlerts.find(a=>a.id?.includes("tg"))?.message || ""} />
          </div>
          {#if mailTgOk && !editMailTg}
            <div class="connected-info">
              <div class="info-item"><span class="info-label">봇 토큰</span><strong>{maskToken(mailTgToken)}</strong></div>
              <div class="info-item"><span class="info-label">채팅 ID</span><strong>{mailTgChatId}</strong></div>
              <div class="btn-row">
                <button class="btn btn-outline btn-sm" onclick={testMailTg} disabled={testingMailTg}>{testingMailTg ? "발송 중..." : "테스트 발송"}</button>
                <button class="btn btn-ghost-sm" onclick={() => editMailTg = true}>재설정</button>
                <button class="btn btn-ghost-sm danger-text" onclick={disconnMailTg}>해지</button>
              </div>
            </div>
          {:else}
            <div class="form-group">
              <label for="mtg-token">봇 토큰 <Tooltip text="SWORK 알림 봇과 별도로 설정하거나, 동일한 봇 토큰을 공유할 수도 있습니다." /></label>
              <input id="mtg-token" type="text" bind:value={mailTgToken} placeholder="123456:ABC-..." />
            </div>
            <div class="form-group">
              <label for="mtg-chat">채팅 ID</label>
              <div class="input-with-btn">
                <input id="mtg-chat" type="text" bind:value={mailTgChatId} placeholder="채팅 ID" />
                <button class="btn btn-outline btn-sm" onclick={lookupMailChats} disabled={lookingUpMail}>{lookingUpMail ? "조회 중..." : "채팅 조회"}</button>
              </div>
            </div>
            {#if mailChatList.length > 0}
              <div class="chat-list">{#each mailChatList as chat}<button class="chat-item" onclick={() => { mailTgChatId = chat.chat_id; mailChatList = []; }}><span class="chat-type">{chat.type}</span><strong>{chat.title}</strong><span class="chat-id">{chat.chat_id}</span></button>{/each}</div>
            {/if}
            <div class="btn-row">
              <button class="btn btn-primary" onclick={testMailTg} disabled={testingMailTg}>{testingMailTg ? "발송 중..." : "테스트 메시지 발송"}</button>
              <button class="btn btn-outline btn-sm" onclick={saveMailTgOnly}>저장만</button>
              {#if mailTgToken && mailTgChatId}<button class="btn btn-ghost-sm" onclick={() => editMailTg = false}>취소</button>{/if}
            </div>
          {/if}
        </Card>
      </div>
    </div>
  {/if}

  <!-- Desk 탭 -->
  {#if activeTab === "desk"}
    <div class="tab-content">
      <Card title="D-Mate Desk 서버">
        {#if deskConnected}
          <div class="connected-info">
            <div class="desk-status">
              <span class="status-dot" class:green={deskReachable} class:red={!deskReachable}></span>
              <span>{deskReachable ? "연결됨" : "서버 응답 없음"}</span>
            </div>
            <div class="info-item"><span class="info-label">서버</span><strong>{deskServerUrl}</strong></div>
            <div class="btn-row mt-12">
              <button class="btn btn-outline btn-sm" onclick={checkDeskHealth}>상태 확인</button>
              <button class="btn btn-ghost-sm danger-text" onclick={() => {
                showDialog({ type: "danger", title: "Desk 연결 해제", message: "Desk 서버와의 연결을 해제합니다.\n저장된 인증 정보가 삭제됩니다.", confirmText: "해제",
                  async onConfirm() {
                    try { await deskDisconnect(); deskConnected = false; deskReachable = false; deskServerUrl = ""; deskName = ""; deskDeviceName = ""; deskState.set({ connected: false, reachable: false, serverUrl: "", joinPending: false, joinRequestId: null, checked: true }); showToast("Desk 연결이 해제되었습니다.", "info"); }
                    catch (e) { showToast("해제 실패", "error"); }
                  },
                });
              }}>연결 해제</button>
            </div>
          </div>
        {:else if joinPending}
          <div class="join-pending">
            <div class="pending-icon-row">
              <span class="pending-spinner"></span>
              <span class="pending-text">승인 대기 중...</span>
            </div>
            <p class="pending-desc">관리자가 참여 요청을 승인하면 자동으로 연결됩니다.</p>
            <div class="btn-row mt-12">
              <button class="btn btn-ghost-sm danger-text" onclick={cancelJoinRequest}>요청 취소</button>
            </div>
          </div>
        {:else}
          <div class="form-group"><label for="desk-url">서버 주소</label><input id="desk-url" type="text" bind:value={deskServerUrl} placeholder="http://192.168.204.53:29180" /></div>
          <div class="form-group"><label for="desk-name">이름</label><input id="desk-name" type="text" bind:value={deskName} placeholder="홍길동" /></div>
          <div class="form-group"><label for="desk-device">디바이스명</label><input id="desk-device" type="text" bind:value={deskDeviceName} placeholder="사무실-PC" /></div>
          <div class="btn-row">
            <button class="btn btn-primary" onclick={requestJoin} disabled={joiningDesk}>{joiningDesk ? "요청 중..." : "참여 요청"}</button>
          </div>
        {/if}
      </Card>
    </div>
  {/if}
</div>

<style>
  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 2px solid var(--c-border);
    margin-bottom: 20px;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 20px;
    background: none;
    border: none;
    font-size: 13px;
    font-weight: 600;
    color: var(--c-text-secondary);
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .tab:hover { color: var(--c-text); }
  .tab.active {
    color: var(--c-primary);
    border-bottom-color: var(--c-primary);
  }
  .tab-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .tab-dot.green { background: var(--c-success); }
  .tab-dot.red { background: var(--c-danger); }
  .tab-dot.gray { background: #d1d5db; }
  .tab-dot.yellow { background: var(--c-warning, #f59e0b); }
  .tab-content {
    animation: fadeIn 0.15s ease;
  }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

  .desk-status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    margin-bottom: 12px;
  }
  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .status-dot.green { background: var(--c-success); }
  .status-dot.red { background: var(--c-danger); }

  .join-pending {
    text-align: center;
    padding: 20px 0;
  }
  .pending-icon-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    margin-bottom: 8px;
  }
  .pending-text {
    font-size: 15px;
    font-weight: 600;
    color: var(--c-text);
  }
  .pending-desc {
    font-size: 13px;
    color: var(--c-text-secondary);
    margin: 0;
  }
  .pending-spinner {
    width: 18px;
    height: 18px;
    border: 2px solid var(--c-border);
    border-top-color: var(--c-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
