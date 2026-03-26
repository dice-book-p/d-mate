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
  import FeedbackPage from "./pages/FeedbackPage.svelte";
  import SystemPage from "./pages/SystemPage.svelte";
  import { currentPage, settings } from "./lib/stores.js";
  import { getSettings } from "./lib/api.js";
  import { onMount } from "svelte";

  let page = $state("");
  let loaded = $state(false);

  currentPage.subscribe((v) => (page = v));

  onMount(async () => {
    try {
      const s = await getSettings();
      settings.set(s);
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
    loaded = true;
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
</style>
