<script>
  let { checked = false, onchange = () => {}, label = "" } = $props();
</script>

<label class="toggle-row">
  <span class="toggle-label">{label}</span>
  <button
    class="toggle"
    class:on={checked}
    onclick={() => onchange(!checked)}
    type="button"
    role="switch"
    aria-checked={checked}
  >
    <span class="toggle-knob"></span>
  </button>
</label>

<style>
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    cursor: pointer;
  }
  .toggle-label {
    font-size: 13px;
    color: var(--c-text);
    font-weight: 500;
  }
  .toggle {
    position: relative;
    width: 40px;
    height: 22px;
    border-radius: 11px;
    background: #d1d5db;
    padding: 0;
    transition: background 0.2s;
    flex-shrink: 0;
  }
  .toggle.on {
    background: var(--c-primary);
  }
  .toggle-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
    transition: transform 0.2s;
  }
  .toggle.on .toggle-knob {
    transform: translateX(18px);
  }
</style>
