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
    "mail_notify_enabled", "mail_schedule_type", "mail_interval_min", "mail_times", "mail_use_work_hours",
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

  const hasMailPass = $derived(s.has_mail_password);
  const hasMailTg = $derived(s.mail_tg_token && s.mail_tg_chat_id);
  const banners = $derived.by(() => {
    const list = [];
    if (!hasMailPass || !s.mail_account) list.push({ message: "메일 서버가 연결되지 않았습니다. 알림을 받으려면 먼저 연결이 필요합니다.", linkText: "메일 서버 연결하기" });
    if (!hasMailTg) list.push({ message: "텔레그램 봇이 연결되지 않았습니다.", linkText: "텔레그램 연결하기" });
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
  <h2 class="page-title">메일 알림</h2>

  <ConnBanner items={banners} />

  <Card title="알림 규칙">
    <div class="rules-section">
      <RuleBlock
        label="메일 알림"
        tooltip="새로운 메일 수신을 감지하여 텔레그램으로 알려줍니다."
        enabled={s.mail_notify_enabled === 1}
        scheduleType={s.mail_schedule_type}
        intervalMin={s.mail_interval_min}
        times={s.mail_times}
        useWorkHours={s.mail_use_work_hours === 1}
        intervals={[1, 2, 5, 10]}
        radioName="mail"
        onchange={(e) => ruleChange("mail", e)}
      />
    </div>
  </Card>

  <div class="save-bar">
    <button class="btn btn-primary btn-lg" onclick={save} disabled={saving || !dirty}>
      {saving ? "저장 중..." : dirty ? "설정 저장" : "변경사항 없음"}
    </button>
  </div>
</div>
