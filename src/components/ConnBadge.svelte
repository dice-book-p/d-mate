<script>
  let { state = "not_configured", error = "", account = "" } = $props();

  const labels = {
    not_configured: "미설정",
    unknown: "확인 전",
    connected: "연결됨",
    auth_error: "인증 오류",
    server_error: "서버 오류",
  };

  const icons = {
    not_configured: "○",
    unknown: "◎",
    connected: "●",
    auth_error: "✕",
    server_error: "△",
  };
</script>

<div class="conn-badge badge-{state}">
  <span class="badge-icon">{icons[state] || "○"}</span>
  <span class="badge-label">{labels[state] || state}</span>
  {#if account && (state === "connected" || state === "unknown")}
    <span class="badge-account">{account}</span>
  {/if}
  {#if error && (state === "auth_error" || state === "server_error")}
    <span class="badge-error">{error}</span>
  {/if}
</div>

<style>
  .conn-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }
  .badge-icon { font-size: 10px; }
  .badge-account {
    color: inherit;
    opacity: 0.7;
    font-family: monospace;
    font-size: 11px;
  }
  .badge-error {
    font-size: 11px;
    opacity: 0.8;
  }

  .badge-not_configured { background: #f3f4f6; color: #9ca3af; }
  .badge-unknown { background: #fef3c7; color: #92400e; }
  .badge-connected { background: #ecfdf5; color: #065f46; }
  .badge-auth_error { background: #fef2f2; color: #991b1b; }
  .badge-server_error { background: #fff7ed; color: #9a3412; }
</style>
