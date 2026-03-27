<script>
  import "./assets/global.css";
  import Sidebar from "./components/Sidebar.svelte";
  import Toast from "./components/Toast.svelte";
  import ConfirmDialog from "./components/ConfirmDialog.svelte";
  import Confetti from "./components/Confetti.svelte";
  import Dashboard from "./pages/Dashboard.svelte";
  import ConnectionManager from "./pages/ConnectionManager.svelte";
  import WorkerAlerts from "./pages/WorkerAlerts.svelte";
  import ManagerAlerts from "./pages/ManagerAlerts.svelte";
  import MailAlerts from "./pages/MailAlerts.svelte";
  import MessagePage from "./pages/MessagePage.svelte";
  import ChatPage from "./pages/ChatPage.svelte";
  import FeedbackPage from "./pages/FeedbackPage.svelte";
  import SystemPage from "./pages/SystemPage.svelte";
  import { currentPage, settings, unreadCount } from "./lib/stores.js";
  import { getSettings, getUnreadCount, checkUpdate } from "./lib/api.js";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  let page = $state("");
  let loaded = $state(false);
  let forceUpdate = $state(null);
  let updating = $state(false);

  currentPage.subscribe((v) => (page = v));

  async function doForceUpdate() {
    updating = true;
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const { relaunch } = await import("@tauri-apps/plugin-process");
      const update = await check();
      if (update) {
        await update.downloadAndInstall();
        await relaunch();
      } else if (forceUpdate?.download_url) {
        const { openUrl } = await import("@tauri-apps/plugin-opener");
        await openUrl(forceUpdate.download_url);
      }
    } catch (e) {
      console.error("Force update failed:", e);
      if (forceUpdate?.download_url) {
        try {
          const { openUrl } = await import("@tauri-apps/plugin-opener");
          await openUrl(forceUpdate.download_url);
        } catch (_) {}
      }
    }
    updating = false;
  }

  onMount(async () => {
    try {
      const s = await getSettings();
      settings.set(s);
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
    // 미읽 메시지 수 로드
    try {
      const r = await getUnreadCount();
      unreadCount.set(r.count || 0);
    } catch (e) { /* ignore */ }
    loaded = true;

    // Desk 서버 업데이트 이벤트 수신 (lib.rs에서 emit)
    listen("update:force", (event) => {
      forceUpdate = event.payload;
    });
    listen("update:available", (_event) => {
      // 선택 업데이트는 SystemPage에서 처리하므로 여기서는 무시
    });

    // 앱 시작 5초 후 업데이트 체크 (Desk 이벤트 미수신 시 폴백)
    setTimeout(async () => {
      try {
        const res = await checkUpdate();
        if (res?.force) {
          forceUpdate = res;
        }
      } catch (_) {}
    }, 5000);
  });
</script>

{#if loaded}
  <div class="app-layout">
    <Sidebar />
    <main class="main-content">
      {#if page === "dashboard"}
        <Dashboard />
      {:else if page === "connection"}
        <ConnectionManager />
      {:else if page === "worker_alerts"}
        <WorkerAlerts />
      {:else if page === "manager_alerts"}
        <ManagerAlerts />
      {:else if page === "mail_alerts"}
        <MailAlerts />
      {:else if page === "message"}
        <MessagePage />
      {:else if page === "chat"}
        <ChatPage />
      {:else if page === "feedback"}
        <FeedbackPage />
      {:else if page === "system"}
        <SystemPage />
      {/if}
    </main>
  </div>
  <Toast />
  <ConfirmDialog />
  <Confetti />

  {#if forceUpdate}
    <div class="force-update-overlay">
      <div class="force-update-modal">
        <div class="update-icon-wrap">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--c-primary)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/>
            <polyline points="7 10 12 15 17 10"/>
            <line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
        </div>
        <h2>필수 업데이트</h2>
        <p class="update-version">v{forceUpdate.latest} 업데이트가 필요합니다.</p>
        {#if forceUpdate.notes}
          <p class="update-notes">{forceUpdate.notes}</p>
        {/if}
        <p class="update-warn">이 업데이트를 설치해야 D-Mate를 사용할 수 있습니다.</p>
        <button class="btn btn-primary btn-lg" onclick={doForceUpdate} disabled={updating}>
          {updating ? "업데이트 중..." : "업데이트 설치"}
        </button>
      </div>
    </div>
  {/if}
{:else}
  <div class="loading-screen">
    <div class="spinner"></div>
    <p>D-Mate 로딩 중...</p>
  </div>
{/if}

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  .main-content {
    flex: 1;
    overflow-y: auto;
    padding: 28px 32px;
    background: var(--c-bg);
  }
  .loading-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    gap: 16px;
    color: var(--c-text-secondary);
  }
  @media (max-width: 768px) {
    .main-content {
      padding: 20px 16px;
    }
  }

  /* Force update overlay */
  .force-update-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    z-index: 9999;
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(4px);
  }
  .force-update-modal {
    background: var(--c-bg);
    border-radius: 16px;
    padding: 40px;
    max-width: 400px;
    width: 90%;
    text-align: center;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  }
  .force-update-modal .update-icon-wrap {
    margin-bottom: 16px;
  }
  .force-update-modal h2 {
    font-size: 20px;
    font-weight: 700;
    margin: 0 0 8px;
    color: var(--c-text);
  }
  .force-update-modal .update-version {
    font-size: 14px;
    color: var(--c-text-secondary);
    margin: 0 0 8px;
  }
  .force-update-modal .update-notes {
    font-size: 13px;
    color: var(--c-text-muted);
    margin: 0 0 12px;
    line-height: 1.5;
  }
  .force-update-modal .update-warn {
    font-size: 13px;
    color: var(--c-danger, #ef4444);
    font-weight: 500;
    margin: 0 0 20px;
  }
</style>
