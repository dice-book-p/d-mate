<script>
  let { text = "" } = $props();
  let show = $state(false);
  let triggerEl = $state(null);
  let popupStyle = $state("");

  function onEnter() {
    show = true;
    if (triggerEl) {
      const rect = triggerEl.getBoundingClientRect();
      const popupWidth = 280;
      let left = rect.left + rect.width / 2 - popupWidth / 2;

      // 화면 왼쪽 넘침 방지
      if (left < 8) left = 8;
      // 화면 오른쪽 넘침 방지
      if (left + popupWidth > window.innerWidth - 8) {
        left = window.innerWidth - popupWidth - 8;
      }

      popupStyle = `top: ${rect.top - 8}px; left: ${left}px; width: ${popupWidth}px;`;
    }
  }
</script>

<span
  class="tooltip-wrap"
  role="note"
  onmouseenter={onEnter}
  onmouseleave={() => (show = false)}
>
  <span class="tooltip-trigger" bind:this={triggerEl}>?</span>
</span>

{#if show}
  <div class="tooltip-popup" style={popupStyle}>
    <div class="tooltip-arrow"></div>
    <p class="tooltip-text">{text}</p>
  </div>
{/if}

<style>
  .tooltip-wrap {
    display: inline-flex;
    align-items: center;
    cursor: help;
  }
  .tooltip-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #e5e7eb;
    color: #6b7280;
    font-size: 11px;
    font-weight: 700;
    transition: background 0.15s;
  }
  .tooltip-trigger:hover {
    background: #d1d5db;
    color: #374151;
  }
  .tooltip-popup {
    position: fixed;
    transform: translateY(-100%);
    background: #1e293b;
    color: #f1f5f9;
    font-size: 12px;
    line-height: 1.6;
    padding: 10px 14px;
    border-radius: 8px;
    z-index: 9999;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.16);
    animation: tooltipIn 0.15s ease;
    pointer-events: none;
  }
  .tooltip-arrow {
    position: absolute;
    bottom: -5px;
    left: 50%;
    transform: translateX(-50%);
    width: 10px;
    height: 10px;
    background: #1e293b;
    border-radius: 0 0 2px 0;
    transform: translateX(-50%) rotate(45deg);
  }
  .tooltip-text {
    margin: 0;
    word-break: keep-all;
    overflow-wrap: break-word;
  }
  @keyframes tooltipIn {
    from {
      opacity: 0;
      transform: translateY(calc(-100% + 4px));
    }
    to {
      opacity: 1;
      transform: translateY(-100%);
    }
  }
</style>
