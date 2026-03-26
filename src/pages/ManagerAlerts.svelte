<script>
  import Card from "../components/Card.svelte";
  import Toggle from "../components/Toggle.svelte";
  import RuleBlock from "../components/RuleBlock.svelte";
  import ConnBanner from "../components/ConnBanner.svelte";
  import { settings, showToast, alerts as alertStore, pageDirty } from "../lib/stores.js";
  import { isDirty, snapshot } from "../lib/dirty.js";
  import { getSettings, saveSettings } from "../lib/api.js";

  let s = $state({});
  let original = $state({});
  let saving = $state(false);

  const DIRTY_FIELDS = [
    "task_notify_enabled",
    "approval_request_enabled", "approval_request_schedule_type", "approval_request_interval_min", "approval_request_times", "approval_request_use_work_hours",
    "overdue_task_enabled", "overdue_task_schedule_type", "overdue_task_interval_min", "overdue_task_times", "overdue_task_use_work_hours",
  ];

  const dirty = $derived(isDirty(original, s, DIRTY_FIELDS));
  $effect(() => { pageDirty.set(dirty); });

  settings.subscribe((v) => {
    if (v && Object.keys(v).length) {
      s = { ...v };
      original = snapshot(v, DIRTY_FIELDS);
    }
  });

  let allAlerts = $state([]);
  alertStore.subscribe((a) => { allAlerts = a || []; });

  const hasSworkPass = $derived(s.has_swork_password);
  const hasSworkTg = $derived(s.swork_tg_token && s.swork_tg_chat_id);
  const banners = $derived.by(() => {
    const list = [];
    if (!hasSworkPass) list.push({ message: "SWORK 계정이 연결되지 않았습니다. 알림을 받으려면 먼저 연결이 필요합니다.", linkText: "SWORK 연결하기" });
    if (!hasSworkTg) list.push({ message: "텔레그램 봇이 연결되지 않았습니다.", linkText: "텔레그램 연결하기" });
    return list;
  });

  function ruleChange(prefix, { field, value }) {
    const key = `${prefix}_${field}`;
    if (field === "enabled" || field === "use_work_hours") {
      s[key] = value ? 1 : 0;
    } else if (field === "interval_min") {
      s[key] = value;
    } else {
      s[key] = value;
    }
  }

  async function save() {
    saving = true;
    try {
      const data = {};
      for (const f of DIRTY_FIELDS) { data[f] = s[f]; }
      await saveSettings(data);
      const fresh = await getSettings();
      settings.set(fresh);
      pageDirty.set(false);
      showToast("저장되었습니다.", "success");
    } catch (e) { showToast("저장 실패", "error"); }
    saving = false;
  }
</script>

<div class="page">
  <h2 class="page-title">관리 업무 알림</h2>

  <ConnBanner items={banners} />

  <Card title="알림 규칙">
    <div class="rules-section">
      <Toggle label="업무 알림 전체" checked={s.task_notify_enabled === 1} onchange={(v) => s.task_notify_enabled = v ? 1 : 0} />

      <div class="rules-list">
        <RuleBlock
          label="승인/검수 요청 알림"
          tooltip="나에게 할당된 승인요청/검수요청 업무를 체크하여 알려줍니다. 처리될 때까지 매 주기마다 반복 알림됩니다."
          enabled={s.approval_request_enabled === 1}
          scheduleType={s.approval_request_schedule_type}
          intervalMin={s.approval_request_interval_min}
          times={s.approval_request_times}
          useWorkHours={s.approval_request_use_work_hours === 1}
          radioName="approval_request"
          onchange={(e) => ruleChange("approval_request", e)}
        />

        <RuleBlock
          label="지연 업무 알림"
          tooltip="내가 지시한 업무 중 마감일이 지난 업무를 알려줍니다. 하루에 1회만 발송됩니다."
          enabled={s.overdue_task_enabled === 1}
          scheduleType={s.overdue_task_schedule_type}
          intervalMin={s.overdue_task_interval_min}
          times={s.overdue_task_times}
          useWorkHours={s.overdue_task_use_work_hours === 1}
          radioName="overdue_task"
          onchange={(e) => ruleChange("overdue_task", e)}
        />
      </div>
    </div>
  </Card>

  <div class="save-bar">
    <button class="btn btn-primary btn-lg" onclick={save} disabled={saving || !dirty}>
      {saving ? "저장 중..." : dirty ? "설정 저장" : "변경사항 없음"}
    </button>
  </div>
</div>

<style>
  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 16px;
  }
</style>
