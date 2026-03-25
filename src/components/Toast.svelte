<script>
  import { toastMessage } from "../lib/stores.js";

  let toast = $state(null);

  toastMessage.subscribe((v) => (toast = v));
</script>

{#if toast}
  <div class="toast toast-{toast.type}">
    <span class="toast-icon">
      {#if toast.type === "success"}✅{:else if toast.type === "error"}❌{:else}ℹ️{/if}
    </span>
    <span>{toast.message}</span>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 24px;
    right: 24px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border-radius: var(--radius-md);
    font-size: 13px;
    font-weight: 500;
    box-shadow: var(--shadow-lg);
    z-index: 9999;
    animation: slideIn 0.25s ease;
  }
  .toast-success {
    background: var(--c-success-light);
    color: #065f46;
    border: 1px solid #a7f3d0;
  }
  .toast-error {
    background: var(--c-danger-light);
    color: #991b1b;
    border: 1px solid #fecaca;
  }
  .toast-info {
    background: var(--c-primary-light);
    color: #1e40af;
    border: 1px solid #bfdbfe;
  }
  .toast-icon {
    font-size: 15px;
  }
  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
