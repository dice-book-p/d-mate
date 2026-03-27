<script>
  import Card from "../components/Card.svelte";
  import { getDashboardData, getAlerts, triggerCheckNow, togglePause } from "../lib/api.js";
  import { showToast, currentPage, alerts as alertStore, pageDirty, navigateTo } from "../lib/stores.js";
  import { checkOvertime } from "../lib/easter.js";
  import { onMount } from "svelte";

  let data = $state(null);
  let activeAlerts = $state([]);
  let loading = $state(true);
  let paused = $state(false);
  let showAllLogs = $state(false);

  onMount(async () => {
    await refresh();
    tryOvertimeCheck();
  });

  function tryOvertimeCheck() {
    const lastShown = sessionStorage.getItem("overtime_shown");
    const now = Date.now();
    if (lastShown && now - Number(lastShown) < 3600000) return;
    const msg = checkOvertime();
    if (msg) {
      setTimeout(() => showToast(msg, "info"), 1500);
      sessionStorage.setItem("overtime_shown", String(now));
    }
  }

  async function refresh() {
    loading = true;
    try {
      data = await getDashboardData();
      paused = data.is_paused;
      const alertData = await getAlerts();
      activeAlerts = alertData.alerts || [];
      alertStore.set(activeAlerts);
    } catch (e) {
      console.error(e);
    }
    loading = false;
  }

  function goTo(action) {
    const page = action.replace("navigate:", "");
    if (page) navigateTo(page);
  }

  async function checkNow() {
    try {
      await triggerCheckNow();
      showToast("확인을 시작했습니다.", "success");
      setTimeout(refresh, 3000);
    } catch (e) {
      showToast("오류가 발생했습니다.", "error");
    }
  }

  async function doPause() {
    try {
      const r = await togglePause();
      paused = r.paused;
      showToast(paused ? "일시 중지됨" : "재개됨", "info");
    } catch (e) {
      showToast("오류가 발생했습니다.", "error");
    }
  }

  function ruleTypeLabel(rt) {
    const map = {
      approval_request: "승인요청",
      overdue_task: "관리지연",
      my_overdue: "내 지연",
      my_deadline: "마감임박",
      mail: "메일",
    };
    return map[rt] || rt;
  }
</script>

<div class="page">
  <div class="page-header">
    <h2>대시보드</h2>
    <div class="header-actions">
      <button class="btn btn-outline" onclick={doPause}>
        {paused ? "▶ 재개" : "⏸ 일시중지"}
      </button>
      <button class="btn btn-primary" onclick={checkNow} title="모든 알림을 즉시 확인합니다">
        📡 알림 발송
      </button>
      <button class="btn btn-outline" onclick={refresh} title="대시보드 데이터를 새로고침합니다">
        🔄 새로고침
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>데이터를 불러오는 중...</p>
    </div>
  {:else if data}
    {#if activeAlerts.length > 0}
      <div class="alert-banner">
        <div class="alert-banner-header">
          <span class="alert-banner-icon">⚠</span>
          <strong>{activeAlerts.length}건의 문제가 감지되었습니다</strong>
        </div>
        <div class="alert-list">
          {#each activeAlerts as alert}
            <button class="alert-item alert-{alert.level}" onclick={() => goTo(alert.action)}>
              <span class="alert-dot"></span>
              <div class="alert-content">
                <strong>{alert.title}</strong>
                <span>{alert.message}</span>
              </div>
              <span class="alert-go">→</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="stats-grid">
      <div class="stat-card stat-red">
        <div class="stat-value">{data.my_overdue_tasks?.length ?? 0}</div>
        <div class="stat-label">내 지연업무</div>
      </div>
      <div class="stat-card stat-orange">
        <div class="stat-value">{data.my_deadline_tasks?.length ?? 0}</div>
        <div class="stat-label">마감임박</div>
      </div>
      <div class="stat-card stat-blue">
        <div class="stat-value">{data.approval_request_tasks?.length ?? 0}</div>
        <div class="stat-label">관리 요청</div>
      </div>
      <div class="stat-card stat-purple">
        <div class="stat-value">{data.overdue_task_tasks?.length ?? 0}</div>
        <div class="stat-label">관리 지연</div>
      </div>
    </div>

    <div class="grid-2">
      <Card title="👤 내 업무 현황">
        {@const myTasks = [...(data.my_overdue_tasks || []), ...(data.my_deadline_tasks || [])]}
        {#if myTasks.length > 0}
          <div class="task-list">
            {#each myTasks as t}
              <div class="task-row">
                {#if t._days_overdue != null}
                  <span class="badge badge-red">{t._days_overdue}일 초과</span>
                {:else if t._days_left != null}
                  <span class="badge badge-orange">{t._days_left === 0 ? "D-day" : "D-1"}</span>
                {/if}
                <div class="task-info">
                  <strong>[{t.t_code}] {t.t_title}</strong>
                  <small>{t.project_title} &middot; {t.t_status}</small>
                  {#if t.t_due_date}<small>기한: {t.t_due_date}</small>{/if}
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <p class="empty-state">지연 또는 마감임박 업무가 없습니다.</p>
        {/if}
      </Card>

      <Card title="📋 승인/검수 요청">
        {#if data.approval_request_tasks?.length > 0}
          <div class="task-list">
            {#each data.approval_request_tasks as t}
              <div class="task-row">
                <span class="badge badge-blue">{t.t_status}</span>
                <div class="task-info">
                  <strong>[{t.t_code}] {t.t_title}</strong>
                  <small>{t.project_title} &middot; {t.assignee_nickname} → {t.assigner_nickname}</small>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <p class="empty-state">요청 대기 중인 업무가 없습니다.</p>
        {/if}
      </Card>
    </div>

    {#if data.overdue_task_tasks?.length > 0}
      <Card title="⏰ 관리 지연 업무">
        <div class="task-list">
          {#each data.overdue_task_tasks as t}
            <div class="task-row">
              <span class="badge badge-orange">{t._days_overdue ?? 0}일 초과</span>
              <div class="task-info">
                <strong>[{t.t_code}] {t.t_title}</strong>
                <small>{t.project_title} &middot; {t.assignee_nickname} → {t.assigner_nickname}</small>
                <small>기한: {t.t_due_date ?? "-"} &middot; {t.t_status}</small>
              </div>
            </div>
          {/each}
        </div>
      </Card>
    {/if}

    <Card title="📜 최근 알림 로그">
      {#if data.recent_logs?.length > 0}
        <div class="log-table-wrap">
          <table class="log-table">
            <thead>
              <tr>
                <th>유형</th>
                <th>코드</th>
                <th>제목</th>
                <th>시간</th>
                <th>결과</th>
              </tr>
            </thead>
            <tbody>
              {#each (showAllLogs ? data.recent_logs : data.recent_logs.slice(0, 20)) as log}
                <tr>
                  <td><span class="badge badge-sm">{ruleTypeLabel(log.rule_type)}</span></td>
                  <td class="mono">{log.task_code}</td>
                  <td class="ellipsis">{log.task_title}</td>
                  <td class="mono">{log.sent_at}</td>
                  <td>{log.success ? "✅" : "❌"}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        {#if data.recent_logs.length > 20}
          <div class="log-more-wrap">
            <button class="log-more-btn" onclick={() => showAllLogs = !showAllLogs}>
              {showAllLogs ? "접기" : `더 보기 (${data.recent_logs.length - 20}건 더)`}
            </button>
          </div>
        {/if}
      {:else}
        <p class="empty-state">아직 알림 기록이 없습니다.</p>
      {/if}
    </Card>
  {/if}
</div>

<style>
  .page {
    max-width: 1000px;
  }
  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }
  .page-header h2 {
    font-size: 20px;
    font-weight: 700;
  }
  .header-actions {
    display: flex;
    gap: 8px;
  }

  .alert-banner {
    background: #fffbeb;
    border: 1px solid #fbbf24;
    border-radius: var(--radius-md);
    padding: 14px 16px;
    margin-bottom: 16px;
  }
  .alert-banner-header {
    display: flex; align-items: center; gap: 8px;
    margin-bottom: 10px; font-size: 13px; color: #92400e;
  }
  .alert-banner-icon { font-size: 16px; }
  .alert-list { display: flex; flex-direction: column; gap: 6px; }
  .alert-item {
    display: flex; align-items: center; gap: 10px;
    padding: 8px 12px; border-radius: var(--radius-sm);
    text-align: left; width: 100%; transition: background 0.15s;
    background: transparent;
  }
  .alert-item:hover { background: rgba(0,0,0,0.04); }
  .alert-warning { color: #92400e; }
  .alert-error { color: #991b1b; }
  .alert-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .alert-warning .alert-dot { background: #f59e0b; }
  .alert-error .alert-dot { background: #ef4444; }
  .alert-content { flex: 1; display: flex; flex-direction: column; gap: 1px; }
  .alert-content strong { font-size: 13px; }
  .alert-content span { font-size: 11px; opacity: 0.75; }
  .alert-go { font-size: 14px; opacity: 0.4; flex-shrink: 0; }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 20px;
  }
  .stat-card {
    background: var(--c-surface);
    border: 1px solid var(--c-border);
    border-radius: var(--radius-md);
    padding: 16px 20px;
    text-align: center;
  }
  .stat-value {
    font-size: 24px;
    font-weight: 700;
    margin-bottom: 4px;
  }
  .stat-label {
    font-size: 12px;
    color: var(--c-text-secondary);
  }
  .stat-blue .stat-value { color: var(--c-primary); }
  .stat-orange .stat-value { color: var(--c-warning); }
  .stat-red .stat-value { color: var(--c-danger); }
  .stat-purple .stat-value { color: #7c3aed; }

  .grid-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 20px;
  }

  .task-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .task-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .task-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .task-info strong {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .task-info small {
    color: var(--c-text-secondary);
    font-size: 12px;
  }

  .badge {
    display: inline-block;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .badge-blue { background: var(--c-primary-light); color: var(--c-primary); }
  .badge-orange { background: var(--c-warning-light); color: #92400e; }
  .badge-red { background: #fee2e2; color: #991b1b; }
  .badge-sm { background: #f3f4f6; color: var(--c-text-secondary); }

  .empty-state {
    color: var(--c-text-muted);
    font-size: 13px;
    text-align: center;
    padding: 12px 0;
  }

  .log-table-wrap {
    overflow-x: auto;
  }
  .log-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  .log-table th {
    text-align: left;
    padding: 8px 10px;
    border-bottom: 1px solid var(--c-border);
    color: var(--c-text-secondary);
    font-weight: 600;
  }
  .log-table td {
    padding: 6px 10px;
    border-bottom: 1px solid #f3f4f6;
  }
  .mono { font-family: monospace; font-size: 11px; }
  .ellipsis {
    max-width: 200px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .log-more-wrap {
    display: flex;
    justify-content: center;
    padding: 10px 0 2px;
  }
  .log-more-btn {
    padding: 6px 16px;
    font-size: 12px;
    font-weight: 600;
    color: var(--c-primary);
    background: transparent;
    border: 1px solid var(--c-primary);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .log-more-btn:hover {
    background: var(--c-primary);
    color: #fff;
  }

  @media (max-width: 900px) {
    .grid-2 { grid-template-columns: 1fr; }
  }
  @media (max-width: 768px) {
    .stats-grid { grid-template-columns: repeat(2, 1fr); }
    .page-header { flex-direction: column; align-items: flex-start; gap: 12px; }
    .header-actions { flex-wrap: wrap; }
  }
  @media (max-width: 480px) {
    .stats-grid { grid-template-columns: 1fr 1fr; }
    .stat-value { font-size: 18px; }
    .stat-card { padding: 12px; }
  }
</style>
