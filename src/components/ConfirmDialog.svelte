<script>
  import { dialogState, closeDialog } from "../lib/dialog.js";

  let state = $state(null);
  dialogState.subscribe((v) => (state = v));

  function onConfirm() {
    if (state?.onConfirm) state.onConfirm();
    closeDialog();
  }
</script>

{#if state}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="dialog-overlay" onclick={closeDialog}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="dialog-box" onclick={(e) => e.stopPropagation()}>
      <div class="dialog-icon">
        {#if state.type === "danger"}⚠️{:else}ℹ️{/if}
      </div>
      <h3 class="dialog-title">{state.title}</h3>
      <p class="dialog-message">{state.message}</p>
      <div class="dialog-actions">
        <button class="btn btn-outline" onclick={closeDialog}>취소</button>
        <button
          class="btn {state.type === 'danger' ? 'btn-danger' : 'btn-primary'}"
          onclick={onConfirm}
        >
          {state.confirmText || "확인"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
    animation: fadeIn 0.15s ease;
  }
  .dialog-box {
    background: var(--c-surface);
    border-radius: var(--radius-lg);
    padding: 28px 32px;
    max-width: 380px;
    width: 90%;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.2);
    text-align: center;
    animation: scaleIn 0.15s ease;
  }
  .dialog-icon { font-size: 32px; margin-bottom: 12px; }
  .dialog-title { font-size: 16px; font-weight: 700; color: var(--c-text); margin-bottom: 8px; }
  .dialog-message {
    font-size: 13px; color: var(--c-text-secondary);
    line-height: 1.6; margin-bottom: 20px; white-space: pre-line;
  }
  .dialog-actions { display: flex; gap: 8px; justify-content: center; }
  .dialog-actions .btn { min-width: 80px; justify-content: center; }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes scaleIn { from { opacity: 0; transform: scale(0.95); } to { opacity: 1; transform: scale(1); } }
</style>
