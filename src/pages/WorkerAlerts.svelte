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
    "my_overdue_enabled", "my_overdue_schedule_type", "my_overdue_interval_min", "my_overdue_times", "my_overdue_use_work_hours",
    "my_deadline_enabled", "my_deadline_schedule_type", "my_deadline_interval_min", "my_deadline_times", "my_deadline_use_work_hours",
  ];

  const dirty = $derived(isDirty(original, s, DIRTY_FIELDS));
  $effect(() => { pageDirty.set(dirty); });

  settings.subscribe((v) => {
    if (v && Object.keys(v).length) {
      s = { ...v };
      original = snapshot(v, DIRTY_FIELDS);
    }
  });

  // 미연결 배너
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
  <h2 class="page-title">내 업무 알림</h2>

  <ConnBanner items={banners} />

  <Card title="알림 규칙">
    <div class="rules-section">
      <Toggle label="업무 알림 전체" checked={s.task_notify_enabled === 1} onchange={(v) => s.task_notify_enabled = v ? 1 : 0} />

      <div class="rules-list">
        <RuleBlock
          label="내 지연업무 알림"
          tooltip="내가 담당자인 업무 중 마감일이 지난 업무를 알려줍니다."
          enabled={s.my_overdue_enabled === 1}
          scheduleType={s.my_overdue_schedule_type}
          intervalMin={s.my_overdue_interval_min}
          times={s.my_overdue_times}
          useWorkHours={s.my_overdue_use_work_hours === 1}
          radioName="my_overdue"
          onchange={(e) => ruleChange("my_overdue", e)}
        />

        <RuleBlock
          label="마감임박 알림"
          tooltip="내가 담당자인 업무 중 마감일이 D-1, D-day인 업무를 알려줍니다."
          enabled={s.my_deadline_enabled === 1}
          scheduleType={s.my_deadline_schedule_type}
          intervalMin={s.my_deadline_interval_min}
          times={s.my_deadline_times}
          useWorkHours={s.my_deadline_use_work_hours === 1}
          radioName="my_deadline"
          onchange={(e) => ruleChange("my_deadline", e)}
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
