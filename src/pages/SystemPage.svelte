<script>
  import Card from "../components/Card.svelte";
  import Toggle from "../components/Toggle.svelte";
  import Tooltip from "../components/Tooltip.svelte";
  import { settings, showToast, pageDirty } from "../lib/stores.js";
  import { showDialog } from "../lib/dialog.js";
  import { isDirty, snapshot } from "../lib/dirty.js";
  import { getSettings, saveSettings, setAutostart, resetAllData, quitApp } from "../lib/api.js";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";

  let s = $state({});
  let saving = $state(false);
  let checking = $state(false);
  let downloading = $state(false);
  let updateInfo = $state(null);
  let downloadProgress = $state(0);
  let original = $state({});

  const DIRTY_FIELDS = [
    "work_hours_enabled", "work_start_time", "work_end_time", "work_days",
    "error_reporting",
  ];

  const dirty = $derived(isDirty(original, s, DIRTY_FIELDS));
  $effect(() => { pageDirty.set(dirty); });

  settings.subscribe((v) => {
    if (v && Object.keys(v).length) {
      s = { ...v };
      original = snapshot(v, DIRTY_FIELDS);
    }
  });

  async function save() {
    saving = true;
    try {
      await saveSettings({
        work_hours_enabled: s.work_hours_enabled,
        work_start_time: s.work_start_time,
        work_end_time: s.work_end_time,
        work_days: s.work_days,
        error_reporting: s.error_reporting,
      });
      const fresh = await getSettings();
      settings.set(fresh);
      pageDirty.set(false);
      showToast("저장되었습니다.", "success");
    } catch (e) {
      showToast("저장 실패", "error");
    }
    saving = false;
  }

  async function toggleAutostart(enabled) {
    try {
      const r = await setAutostart(enabled);
      if (r.ok) {
        s.autostart = enabled ? 1 : 0;
        showToast(r.message, "success");
      } else {
        showToast(r.message, "error");
      }
    } catch (e) { showToast("자동 시작 설정 실패", "error"); }
  }

  async function doCheckUpdate() {
    checking = true;
    try {
      const update = await check();
      if (update) {
        updateInfo = { available: true, latest: update.version, notes: update.body || "", _update: update };
        showToast(`새 버전 v${update.version} 사용 가능!`, "info");
      } else {
        updateInfo = { available: false };
        showToast("최신 버전입니다.", "success");
      }
    } catch (e) {
      console.error("Update check error:", e);
      showToast("업데이트 확인 실패", "error");
    }
    checking = false;
  }

  async function doDownloadUpdate() {
    if (!updateInfo?._update) return;
    downloading = true;
    downloadProgress = 0;
    try {
      let totalLen = 0;
      let downloaded = 0;
      await updateInfo._update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalLen = event.data.contentLength || 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          downloadProgress = totalLen > 0 ? Math.round((downloaded / totalLen) * 100) : 0;
        } else if (event.event === "Finished") {
          downloadProgress = 100;
        }
      });

      // 다운로드 + 설치 완료 → 재시작 물어보기
      downloading = false;
      showDialog({
        type: "info",
        title: "업데이트 준비 완료",
        message: `v${updateInfo.latest} 업데이트가 설치되었습니다.\n지금 재시작하시겠습니까?\n\n나중에 앱을 재시작해도 자동 적용됩니다.`,
        confirmText: "지금 재시작",
        async onConfirm() {
          await relaunch();
        },
      });
    } catch (e) {
      downloading = false;
      showToast("업데이트 다운로드 실패: " + e, "error");
    }
  }

  function doReset() {
    showDialog({
      type: "danger",
      title: "전체 데이터 초기화",
      message: "모든 설정, 자격증명, 알림 로그가 삭제됩니다.\n이 작업은 되돌릴 수 없습니다.",
      confirmText: "초기화",
      async onConfirm() {
        try {
          await resetAllData();
          showToast("초기화 완료. 앱을 재시작합니다.", "info");
          setTimeout(() => location.reload(), 1500);
        } catch (e) {
          showToast("초기화 실패", "error");
        }
      },
    });
  }

  async function doQuit() {
    try {
      await quitApp();
    } catch (e) {
      // app exiting
    }
  }
</script>

<div class="page">
  <h2 class="page-title">시스템 설정</h2>

  <div class="section-grid">
    <Card title="근무시간 설정">
      <Toggle
        label="근무시간 외 알림 차단"
        checked={s.work_hours_enabled === 1}
        onchange={(v) => (s.work_hours_enabled = v ? 1 : 0)}
      />

      <div class="form-row mt-12">
        <div class="form-group flex-1">
          <label>출근 시간</label>
          <input type="time" bind:value={s.work_start_time} />
        </div>
        <div class="form-group flex-1">
          <label>퇴근 시간</label>
          <input type="time" bind:value={s.work_end_time} />
        </div>
      </div>
      <div class="form-group">
        <label>
          근무 요일
          <Tooltip text="요일 범위(mon-fri) 또는 쉼표 구분(mon,tue,wed,thu,fri) 형식으로 입력합니다." />
        </label>
        <input type="text" bind:value={s.work_days} placeholder="mon-fri" />
      </div>
    </Card>

    <Card title="일반">
      <div class="toggle-list">
        <Toggle
          label="PC 부팅 시 자동 시작"
          checked={s.autostart === 1}
          onchange={(v) => toggleAutostart(v)}
        />
        <Toggle
          label="익명 에러 리포팅"
          checked={s.error_reporting === 1}
          onchange={(v) => (s.error_reporting = v ? 1 : 0)}
        />
      </div>

      <div class="info-section mt-12">
        <div class="info-row">
          <span>앱 버전</span>
          <div class="info-row-right">
            <strong>v{s.app_version || "0.1.0"}</strong>
            <button class="btn btn-outline btn-sm" onclick={doCheckUpdate} disabled={checking}>
              {checking ? "확인 중..." : "업데이트 확인"}
            </button>
          </div>
        </div>
        {#if updateInfo?.available}
          <div class="update-notice">
            <div>
              새 버전 <strong>v{updateInfo.latest}</strong> 사용 가능
              {#if updateInfo.notes}
                <span class="update-notes">— {updateInfo.notes}</span>
              {/if}
            </div>
            {#if downloading}
              <div class="progress-bar mt-8">
                <div class="progress-fill" style="width: {downloadProgress}%"></div>
              </div>
              <span class="progress-text">{downloadProgress}% 다운로드 중...</span>
            {:else}
              <button class="btn btn-primary btn-sm mt-8" onclick={doDownloadUpdate}>
                업데이트 설치
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </Card>
  </div>

  <Card title="위험 영역">
    <div class="danger-zone">
      <div class="danger-item">
        <div>
          <strong>데이터 초기화</strong>
          <p>모든 설정과 Keychain 자격증명을 삭제합니다.</p>
        </div>
        <button class="btn btn-danger" onclick={doReset}>
전체 초기화
        </button>
      </div>
      <div class="danger-item">
        <div>
          <strong>앱 종료</strong>
          <p>트레이를 포함하여 앱을 완전히 종료합니다.</p>
        </div>
        <button class="btn btn-outline" onclick={doQuit}>앱 종료</button>
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
  .toggle-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .info-section {
    background: #f9fafb;
    border-radius: var(--radius-sm);
    padding: 12px 14px;
  }
  .info-row {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
  }
  .info-row span { color: var(--c-text-secondary); }
  .info-row-right { display: flex; align-items: center; gap: 8px; }
  .update-notice {
    margin-top: 8px; padding: 8px 10px;
    background: var(--c-primary-light); color: var(--c-primary);
    border-radius: var(--radius-sm); font-size: 12px;
  }
  .update-notes { opacity: 0.7; }
  .progress-bar {
    height: 6px; background: #e5e7eb; border-radius: 3px; overflow: hidden;
  }
  .progress-fill {
    height: 100%; background: var(--c-primary); border-radius: 3px;
    transition: width 0.3s;
  }
  .progress-text { font-size: 11px; color: var(--c-text-secondary); margin-top: 4px; display: block; }

  .danger-zone {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .danger-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 0;
    border-bottom: 1px solid #f3f4f6;
  }
  .danger-item:last-child { border-bottom: none; }
  .danger-item strong { font-size: 13px; }
  .danger-item p {
    font-size: 12px;
    color: var(--c-text-muted);
    margin-top: 2px;
  }
</style>
