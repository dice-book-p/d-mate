<script>
  import { currentPage, settings, alerts, navigateTo, showToast, unreadCount } from "../lib/stores.js";
  import { triggerConfetti } from "../lib/easter.js";

  let page = $state("");
  let collapsed = $state(false);
  let appVersion = $state("");
  let alertCount = $state(0);
  let alertSources = $state(new Set());
  let msgUnread = $state(0);

  unreadCount.subscribe((c) => (msgUnread = c));

  currentPage.subscribe((v) => (page = v));
  settings.subscribe((s) => {
    appVersion = s?.app_version || "0.1.0";
  });
  alerts.subscribe((a) => {
    alertCount = a?.length || 0;
    alertSources = new Set((a || []).map(al => {
      if (al.source?.startsWith("swork")) return "swork";
      if (al.source?.startsWith("mail")) return "mail";
      return "system";
    }));
  });

  const nav = [
    { id: "dashboard", icon: "📊", label: "대시보드" },
    { id: "worker_alerts", icon: "👤", label: "내 업무 알림", group: "swork" },
    { id: "manager_alerts", icon: "📋", label: "관리 업무 알림", group: "swork" },
    { id: "mail_alerts", icon: "📬", label: "메일 알림", group: "mail" },
    { id: "message", icon: "💬", label: "메시지" },
    { id: "feedback", icon: "📝", label: "피드백" },
    { id: "connection", icon: "🔗", label: "연결 관리" },
    { id: "system", icon: "⚙️", label: "시스템" },
  ];

  let logoClicks = $state(0);
  let logoTimer = null;

  function onLogoClick() {
    logoClicks++;
    clearTimeout(logoTimer);
    if (logoClicks >= 7) {
      logoClicks = 0;
      const msg = triggerConfetti();
      showToast(msg, "success");
    } else {
      logoTimer = setTimeout(() => logoClicks = 0, 2000);
    }
  }

  function go(item) {
    if (item.disabled) return;
    navigateTo(item.id);
  }
</script>

<aside class="sidebar" class:collapsed>
  <div class="sidebar-header">
    {#if !collapsed}
      <div class="logo" onclick={onLogoClick}>
        <span class="logo-icon">🤝</span>
        <div class="logo-text">
          <strong>D-Mate</strong>
          <small>업무 도우미</small>
        </div>
      </div>
    {:else}
      <span class="logo-icon collapsed-icon" onclick={onLogoClick}>🤝</span>
    {/if}
  </div>

  <nav class="sidebar-nav">
    {#each nav as item}
      <button
        class="nav-item"
        class:active={page === item.id}
        class:disabled={item.disabled}
        onclick={() => go(item)}
        title={item.disabled ? `${item.label} (준비 중)` : item.label}
      >
        <span class="nav-icon">
          {item.icon}
          {#if item.group && alertSources.has(item.group)}
            <span class="nav-alert-dot"></span>
          {/if}
        </span>
        {#if !collapsed}
          <span class="nav-label">{item.label}</span>
          {#if item.id === "dashboard" && alertCount > 0}
            <span class="nav-badge">{alertCount}</span>
          {/if}
          {#if item.id === "message" && msgUnread > 0}
            <span class="nav-badge">{msgUnread}</span>
          {/if}
          {#if item.disabled}
            <span class="nav-soon">준비 중</span>
          {/if}
        {/if}
      </button>
    {/each}
  </nav>

  <div class="sidebar-footer">
    <button class="collapse-btn" onclick={() => (collapsed = !collapsed)} title="사이드바 접기">
      {collapsed ? "▶" : "◀"}
    </button>
    {#if !collapsed}
      <span class="version">v{appVersion}</span>
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    width: 220px;
    min-width: 220px;
    background: var(--c-sidebar);
    display: flex;
    flex-direction: column;
    transition: width 0.2s ease, min-width 0.2s ease;
    user-select: none;
  }
  .sidebar.collapsed {
    width: 60px;
    min-width: 60px;
  }

  .sidebar-header {
    padding: 20px 16px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }
  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .logo-icon {
    font-size: 24px;
  }
  .collapsed-icon {
    display: block;
    text-align: center;
  }
  .logo-text {
    display: flex;
    flex-direction: column;
  }
  .logo-text strong {
    color: #fff;
    font-size: 16px;
    letter-spacing: -0.3px;
  }
  .logo-text small {
    color: var(--c-sidebar-text);
    font-size: 11px;
    margin-top: 1px;
  }

  .sidebar-nav {
    flex: 1;
    padding: 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--c-sidebar-text);
    font-size: 13px;
    font-weight: 500;
    transition: all 0.15s;
    width: 100%;
    text-align: left;
  }
  .nav-item:hover:not(.disabled) {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }
  .nav-item.active {
    background: var(--c-sidebar-active);
    color: #fff;
  }
  .nav-item.disabled {
    opacity: 0.4;
    cursor: default;
  }
  .nav-icon {
    font-size: 16px;
    width: 20px;
    text-align: center;
    flex-shrink: 0;
    position: relative;
  }
  .nav-label {
    white-space: nowrap;
    overflow: hidden;
  }
  .nav-alert-dot {
    position: absolute;
    top: -2px;
    right: -4px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #ef4444;
  }
  .nav-badge {
    margin-left: auto;
    background: #ef4444;
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 10px;
    min-width: 18px;
    text-align: center;
  }
  .nav-soon {
    margin-left: auto;
    font-size: 10px;
    color: rgba(255, 255, 255, 0.3);
  }

  .sidebar-footer {
    padding: 12px 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .collapse-btn {
    background: transparent;
    color: var(--c-sidebar-text);
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    font-size: 11px;
  }
  .collapse-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .version {
    color: var(--c-text-muted);
    font-size: 11px;
  }

  .sidebar.collapsed .nav-item {
    justify-content: center;
    padding: 10px;
  }
  .sidebar.collapsed .sidebar-footer {
    justify-content: center;
  }

  @media (max-width: 768px) {
    .sidebar {
      width: 60px;
      min-width: 60px;
    }
    .logo-text,
    .nav-label,
    .version,
    .nav-soon {
      display: none;
    }
    .nav-item {
      justify-content: center;
      padding: 10px;
    }
    .sidebar-footer {
      justify-content: center;
    }
  }
</style>
