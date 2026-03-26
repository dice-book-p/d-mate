<script>
  import Card from "../components/Card.svelte";
  import ConnBanner from "../components/ConnBanner.svelte";
  import { showToast } from "../lib/stores.js";
  import { deskSubmitFeedback, deskGetFeedback, deskHealth } from "../lib/api.js";
  import { onMount } from "svelte";

  let deskConnected = $state(false);
  let feedbackList = $state([]);
  let loading = $state(true);
  let submitting = $state(false);

  // 신규 작성 폼
  let showForm = $state(false);
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
  });

  async function loadFeedback() {
    try {
      const data = await deskGetFeedback();
      if (Array.isArray(data)) feedbackList = data;
      else if (data.error) feedbackList = [];
    } catch (e) { feedbackList = []; }
  }

  async function submit() {
    if (!title.trim() || !body.trim()) {
      showToast("제목과 내용을 입력하세요.", "error"); return;
    }
    submitting = true;
    try {
      const r = await deskSubmitFeedback(category, title.trim(), body.trim());
      if (r.ok) {
        showToast("피드백이 전송되었습니다!", "success");
        title = ""; body = ""; showForm = false;
        await loadFeedback();
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
    {#if deskConnected}
      <button class="btn btn-primary" onclick={() => showForm = !showForm}>
        {showForm ? "취소" : "+ 새 피드백"}
      </button>
    {/if}
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
</style>
