<script>
  import Card from "../components/Card.svelte";
  import ConnBanner from "../components/ConnBanner.svelte";
  import { showToast } from "../lib/stores.js";
  import { deskSubmitFeedback, deskGetFeedback, deskHealth } from "../lib/api.js";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  let deskConnected = $state(false);
  let feedbackList = $state([]);
  let loading = $state(true);
  let submitting = $state(false);

  // 페이지네이션
  const FB_PER_PAGE = 10;
  let fbPage = $state(1);
  let fbTotalPages = $state(1);

  // 신규 작성 폼
  let showForm = $state(false);
  let unlisten = null;
  let category = $state("suggestion");
  let title = $state("");
  let body = $state("");

  const banners = $derived.by(() => {
    if (!deskConnected) return [{ message: "Desk 서버에 연결되지 않았습니다.", linkText: "Desk 연결하기" }];
    return [];
  });

  const categoryLabel = { bug: "버그", suggestion: "건의", question: "질문" };
  const statusLabel = { open: "접수", in_progress: "처리 중", resolved: "해결", closed: "종료" };
  const statusColor = { open: "blue", in_progress: "orange", resolved: "green", closed: "gray" };

  onMount(async () => {
    try {
      const h = await deskHealth();
      deskConnected = h.connected;
      if (deskConnected) await loadFeedback();
    } catch (e) { /* desk not connected */ }
    loading = false;

    // 피드백 상태 변경 실시간 수신
    unlisten = await listen("mqtt:feedback_update", async () => {
      if (deskConnected) await loadFeedback();
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function loadFeedback(page = 1) {
    try {
      const data = await deskGetFeedback(page, FB_PER_PAGE);
      if (data?.error) {
        feedbackList = [];
      } else {
        feedbackList = data.data || [];
        const total = data.total || feedbackList.length;
        fbTotalPages = Math.max(1, Math.ceil(total / FB_PER_PAGE));
      }
      fbPage = page;
    } catch (e) { feedbackList = []; }
  }

  function goFbPage(p) {
    if (p < 1 || p > fbTotalPages || p === fbPage) return;
    loadFeedback(p);
  }

  async function submit() {
    if (!title.trim() || !body.trim()) {
      showToast("제목과 내용을 입력하세요.", "error"); return;
    }
    submitting = true;
    try {
      const r = await deskSubmitFeedback(category, title.trim(), body.trim());
      if (r.ok) {
        // 오프라인 큐에 저장된 경우 별도 안내
        if (r.message && r.message.includes("자동 전송")) {
          showToast(r.message, "info");
        } else {
          showToast("피드백이 전송되었습니다!", "success");
        }
        title = ""; body = ""; showForm = false;
        if (deskConnected) await loadFeedback(1);
      } else {
        showToast(r.message || "전송 실패", "error");
      }
    } catch (e) { showToast("전송 실패: " + e, "error"); }
    submitting = false;
  }
</script>

<div class="page">
  <div class="page-header">
    <h2 class="page-title">피드백</h2>
    <button class="btn btn-primary" onclick={() => showForm = !showForm}>
      {showForm ? "취소" : "+ 새 피드백"}
    </button>
  </div>

  <ConnBanner items={banners} />

  {#if showForm}
    <Card title="피드백 작성">
      <div class="form-group">
        <label for="fb-cat">카테고리</label>
        <select id="fb-cat" bind:value={category}>
          <option value="bug">버그</option>
          <option value="suggestion">건의</option>
          <option value="question">질문</option>
        </select>
      </div>
      <div class="form-group">
        <label for="fb-title">제목</label>
        <input id="fb-title" type="text" bind:value={title} placeholder="간단한 제목" />
      </div>
      <div class="form-group">
        <label for="fb-body">내용</label>
        <textarea id="fb-body" bind:value={body} placeholder="상세 내용을 입력하세요" rows="5"></textarea>
      </div>
      <div class="btn-row">
        <button class="btn btn-primary" onclick={submit} disabled={submitting}>
          {submitting ? "전송 중..." : "전송"}
        </button>
      </div>
    </Card>
  {/if}

  {#if loading}
    <div class="loading-state"><div class="spinner"></div><p>로딩 중...</p></div>
  {:else if !deskConnected}
    <p class="empty-state">Desk 서버에 연결하면 피드백을 보내고 확인할 수 있습니다.</p>
  {:else if feedbackList.length === 0}
    <p class="empty-state">아직 작성한 피드백이 없습니다.</p>
  {:else}
    <div class="feedback-list">
      {#each feedbackList as fb}
        <Card>
          <div class="fb-header">
            <span class="badge badge-{statusColor[fb.status] || 'gray'}">{statusLabel[fb.status] || fb.status}</span>
            <span class="badge badge-outline">{categoryLabel[fb.category] || fb.category}</span>
            <span class="fb-date">{fb.created_at}</span>
          </div>
          <h4 class="fb-title">{fb.title}</h4>
          <p class="fb-body">{fb.body}</p>
          {#if fb.admin_note}
            <div class="fb-reply">
              <strong>관리자 답변</strong>
              <p>{fb.admin_note}</p>
            </div>
          {/if}
        </Card>
      {/each}
    </div>
    {#if fbTotalPages > 1}
      <div class="pagination">
        <button class="page-btn" onclick={() => goFbPage(fbPage - 1)} disabled={fbPage <= 1}>이전</button>
        {#each Array.from({ length: fbTotalPages }, (_, i) => i + 1) as p}
          <button class="page-btn" class:active={p === fbPage} onclick={() => goFbPage(p)}>{p}</button>
        {/each}
        <button class="page-btn" onclick={() => goFbPage(fbPage + 1)} disabled={fbPage >= fbTotalPages}>다음</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
  }
  .feedback-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .fb-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .fb-date { font-size: 11px; color: var(--c-text-muted); margin-left: auto; }
  .fb-title { font-size: 14px; font-weight: 600; margin-bottom: 6px; }
  .fb-body { font-size: 13px; color: var(--c-text-secondary); line-height: 1.6; white-space: pre-wrap; }
  .fb-reply {
    margin-top: 12px;
    padding: 10px 12px;
    background: var(--c-primary-light);
    border-radius: var(--radius-sm);
    font-size: 13px;
  }
  .fb-reply strong { font-size: 12px; color: var(--c-primary); display: block; margin-bottom: 4px; }
  .fb-reply p { color: var(--c-text); }

  .badge-green { background: #dcfce7; color: #166534; }
  .badge-gray { background: #f3f4f6; color: var(--c-text-secondary); }
  .badge-outline { background: transparent; border: 1px solid var(--c-border); color: var(--c-text-secondary); }

  .empty-state {
    color: var(--c-text-muted);
    font-size: 13px;
    text-align: center;
    padding: 40px 0;
  }

  textarea {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--c-border);
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-family: inherit;
    resize: vertical;
    background: var(--c-surface);
    color: var(--c-text);
  }

  /* 페이지네이션 */
  .pagination {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 4px;
    margin-top: 16px;
    padding: 8px 0;
  }
  .page-btn {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--c-text-secondary);
    background: var(--c-surface);
    border: 1px solid var(--c-border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .page-btn:hover:not(:disabled) {
    background: var(--c-primary-light);
    color: var(--c-primary);
  }
  .page-btn.active {
    background: var(--c-primary);
    color: #fff;
    border-color: var(--c-primary);
  }
  .page-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
