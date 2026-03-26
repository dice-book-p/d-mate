<script>
  import Toggle from "./Toggle.svelte";
  import Tooltip from "./Tooltip.svelte";

  let {
    label = "",
    tooltip = "",
    enabled = false,
    scheduleType = "interval",
    intervalMin = 5,
    times = "",
    useWorkHours = true,
    intervals = [2, 5, 10, 15, 30],
    radioName = "rule",
    onchange = () => {},
  } = $props();

  function emit(field, value) {
    onchange({ field, value });
  }
</script>

<div class="rule-block">
  <div class="rule-header">
    <Toggle label={label} checked={enabled} onchange={(v) => emit("enabled", v)} />
    {#if tooltip}
      <Tooltip text={tooltip} />
    {/if}
  </div>

  {#if enabled}
    <div class="rule-body">
      <div class="schedule-selector">
        <label class="radio-label">
          <input type="radio" name={radioName} value="interval" checked={scheduleType === "interval"} onchange={() => emit("schedule_type", "interval")} />
          주기 반복
        </label>
        <label class="radio-label">
          <input type="radio" name={radioName} value="times" checked={scheduleType === "times"} onchange={() => emit("schedule_type", "times")} />
          특정 시간
        </label>
      </div>

      {#if scheduleType === "interval"}
        <div class="schedule-detail">
          <label for="{radioName}-interval">확인 주기</label>
          <select id="{radioName}-interval" value={intervalMin} onchange={(e) => emit("interval_min", Number(e.target.value))}>
            {#each intervals as m}
              <option value={m}>{m}분</option>
            {/each}
          </select>
          <span class="schedule-hint">매 정시 기준 ({intervalMin}분 간격)</span>
        </div>
      {:else}
        <div class="schedule-detail">
          <label for="{radioName}-times">알림 시간 (쉼표 구분)</label>
          <input id="{radioName}-times" type="text" value={times} onchange={(e) => emit("times", e.target.value)} placeholder="08:50,13:20,17:50" />
        </div>
      {/if}

      <div class="work-hours-toggle">
        <Toggle label="근무시간 내에서만 발송" checked={useWorkHours} onchange={(v) => emit("use_work_hours", v)} />
      </div>
    </div>
  {/if}
</div>

<style>
  .rule-block {
    border: 1px solid var(--c-border);
    border-radius: var(--radius-md);
    padding: 14px 16px;
  }
  .rule-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .rule-body {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--c-border);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .schedule-selector {
    display: flex;
    gap: 16px;
  }
  .radio-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--c-text);
    cursor: pointer;
  }
  .schedule-detail {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .schedule-detail label {
    font-size: 12px;
    color: var(--c-text-secondary);
    font-weight: 500;
  }
  .schedule-detail select,
  .schedule-detail input {
    padding: 7px 10px;
    border: 1px solid var(--c-border);
    border-radius: var(--radius-sm);
    font-size: 13px;
    background: var(--c-surface);
    color: var(--c-text);
    max-width: 280px;
  }
  .schedule-hint {
    font-size: 11px;
    color: var(--c-text-muted);
  }
  .work-hours-toggle {
    padding-top: 4px;
  }
</style>
