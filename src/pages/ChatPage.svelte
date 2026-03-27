<script>
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getConversationMessages, sendDm, dmRead, getMessages } from "../lib/api.js";
  import { navigateTo, chatParams } from "../lib/stores.js";

  let params = $state({});
  let messages = $state([]);
  let inputText = $state("");
  let loading = $state(true);
  let sending = $state(false);
  let messagesEnd;
  let messagesContainer;
  let unlisten = null;

  // 페이지네이션 상태
  const CHAT_PAGE_SIZE = 50;
  let chatOffset = $state(0);
  let hasOlder = $state(true);
  let loadingOlder = $state(false);
  // chatParams store에서 값 가져오기
  const unsubParams = chatParams.subscribe((v) => {
    params = v || {};
  });

  onMount(async () => {
    await loadMessages();
    loading = false;
    scrollToBottom();

    // 실시간 수신
    unlisten = await listen("mqtt:message", (event) => {
      const msg = event.payload;
      // 현재 대화의 메시지만 추가
      const convId = params.conv_id;
      const targetCode = params.target_code;
      if (
        msg.conversation_id === convId ||
        msg.sender_code === targetCode ||
        msg.target_code === targetCode
      ) {
        messages = [...messages, msg];
        scrollToBottom();
        // 읽음 처리
        if (msg.id) {
          dmRead(msg.id).catch(() => {});
        }
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    unsubParams();
  });

  async function loadMessages() {
    try {
      if (params.conv_id) {
        const data = await getConversationMessages(params.conv_id, CHAT_PAGE_SIZE, 0);
        let fetched;
        if (data?.data) {
          fetched = data.data;
        } else if (Array.isArray(data)) {
          fetched = data;
        } else {
          // conv_id로 로컬 메시지 fallback
          const local = await getMessages(params.conv_id, CHAT_PAGE_SIZE, 0);
          fetched = Array.isArray(local) ? local : [];
        }
        messages = fetched;
        chatOffset = fetched.length;
        hasOlder = fetched.length >= CHAT_PAGE_SIZE;
      } else {
        messages = [];
      }

      // 읽음 처리
      for (const msg of messages) {
        if (msg.id && !msg.is_read) {
          dmRead(msg.id).catch(() => {});
        }
      }
    } catch (e) {
      console.error("Failed to load chat messages:", e);
      if (params.conv_id) {
        try {
          const local = await getMessages(params.conv_id, CHAT_PAGE_SIZE, 0);
          const fetched = Array.isArray(local) ? local : [];
          messages = fetched;
          chatOffset = fetched.length;
          hasOlder = fetched.length >= CHAT_PAGE_SIZE;
        } catch { messages = []; }
      } else {
        messages = [];
      }
    }
  }

  async function loadOlderMessages() {
    if (loadingOlder || !hasOlder || !params.conv_id) return;
    loadingOlder = true;
    try {
      // 서버 API로 이전 메시지 로드
      let older;
      try {
        const data = await getConversationMessages(params.conv_id, CHAT_PAGE_SIZE, chatOffset);
        if (data?.data) {
          older = data.data;
        } else if (Array.isArray(data)) {
          older = data;
        } else {
          const local = await getMessages(params.conv_id, CHAT_PAGE_SIZE, chatOffset);
          older = Array.isArray(local) ? local : [];
        }
      } catch {
        const local = await getMessages(params.conv_id, CHAT_PAGE_SIZE, chatOffset);
        older = Array.isArray(local) ? local : [];
      }

      if (older.length > 0) {
        // 스크롤 위치 보존
        const container = messagesContainer;
        const prevHeight = container?.scrollHeight || 0;
        messages = [...older, ...messages];
        chatOffset += older.length;
        hasOlder = older.length >= CHAT_PAGE_SIZE;
        // 스크롤 위치 복원 (새로 추가된 만큼 보정)
        requestAnimationFrame(() => {
          if (container) {
            container.scrollTop = container.scrollHeight - prevHeight;
          }
        });
      } else {
        hasOlder = false;
      }
    } catch (e) {
      console.error("Failed to load older messages:", e);
    }
    loadingOlder = false;
  }

  function handleChatScroll(e) {
    const container = e.target;
    if (container.scrollTop < 60 && hasOlder && !loadingOlder) {
      loadOlderMessages();
    }
  }

  async function send() {
    const text = inputText.trim();
    if (!text || sending) return;

    sending = true;
    try {
      const result = await sendDm(params.target_code, text);
      if (result?.ok) {
        // 즉시 UI에 추가 (낙관적 업데이트)
        const now = new Date().toISOString();
        messages = [
          ...messages,
          {
            id: `local-${Date.now()}`,
            conversation_id: params.conv_id,
            sender_code: "__me__",
            sender_name: "나",
            body: text,
            created_at: now,
            type: "dm",
            is_mine: true,
          },
        ];
        inputText = "";
        scrollToBottom();
      }
    } catch (e) {
      console.error("Failed to send DM:", e);
    } finally {
      sending = false;
    }
  }

  function scrollToBottom() {
    requestAnimationFrame(() => {
      messagesEnd?.scrollIntoView({ behavior: "smooth" });
    });
  }

  function formatTime(dateStr) {
    if (!dateStr) return "";
    try {
      // 서버가 KST로 저장하므로 로컬 시간으로 파싱 (Z 미추가)
      const d = new Date(dateStr.replace(" ", "T"));
      return d.toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit" });
    } catch {
      return dateStr;
    }
  }

  function formatDateSeparator(dateStr) {
    if (!dateStr) return "";
    try {
      // 서버가 KST로 저장하므로 로컬 시간으로 파싱 (Z 미추가)
      const d = new Date(dateStr.replace(" ", "T"));
      return d.toLocaleDateString("ko-KR", {
        year: "numeric",
        month: "long",
        day: "numeric",
        weekday: "short",
      });
    } catch {
      return "";
    }
  }

  function isMine(msg) {
    // is_mine 플래그가 있으면 사용, 아니면 sender_code로 판별
    if (msg.is_mine) return true;
    if (msg.sender_code === "__me__") return true;
    // 상대방 코드가 아니면 내 메시지
    if (msg.sender_code && msg.sender_code !== params.target_code) return true;
    return false;
  }

  function needsDateSep(idx) {
    if (idx === 0) return true;
    const cur = messages[idx]?.created_at?.substring(0, 10);
    const prev = messages[idx - 1]?.created_at?.substring(0, 10);
    return cur !== prev;
  }

  function handleKeydown(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function goBack() {
    navigateTo("message");
  }
</script>

<div class="chat-container">
  <header class="chat-header">
    <button class="back-btn" onclick={goBack}>
      <span class="back-arrow">&#8592;</span>
      뒤로
    </button>
    <div class="header-center">
      <span class="header-avatar">
        {(params.target_name || "?").charAt(0)}
      </span>
      <h3 class="header-name">{params.target_name || "대화"}</h3>
    </div>
    <div class="header-spacer"></div>
  </header>

  <div class="chat-messages" bind:this={messagesContainer} onscroll={handleChatScroll}>
    {#if loading}
      <div class="chat-loading">
        <div class="spinner"></div>
        <p>메시지 로딩 중...</p>
      </div>
    {:else if messages.length === 0}
      <div class="chat-empty">
        <p>아직 메시지가 없습니다.</p>
        <small>첫 메시지를 보내보세요!</small>
      </div>
    {:else}
      {#if loadingOlder}
        <div class="loading-older">
          <div class="spinner-sm"></div>
          <span>이전 메시지 로딩 중...</span>
        </div>
      {:else if hasOlder}
        <button class="load-older-btn" onclick={loadOlderMessages}>이전 메시지 불러오기</button>
      {/if}
      {#each messages as msg, idx}
        {#if needsDateSep(idx)}
          <div class="date-separator">
            <span>{formatDateSeparator(msg.created_at)}</span>
          </div>
        {/if}
        <div class="msg-row" class:mine={isMine(msg)} class:theirs={!isMine(msg)}>
          {#if !isMine(msg)}
            <span class="msg-avatar">{(params.target_name || "?").charAt(0)}</span>
          {/if}
          <div class="msg-bubble">
            <div class="msg-body">{msg.body}</div>
            <div class="msg-meta">
              <span class="msg-time">{formatTime(msg.created_at)}</span>
            </div>
          </div>
        </div>
      {/each}
    {/if}
    <div bind:this={messagesEnd}></div>
  </div>

  <div class="chat-input-area">
    <input
      class="chat-input"
      bind:value={inputText}
      placeholder="메시지 입력..."
      onkeydown={handleKeydown}
      disabled={sending}
    />
    <button
      class="send-btn"
      onclick={send}
      disabled={!inputText.trim() || sending}
    >
      {sending ? "..." : "전송"}
    </button>
  </div>
</div>

<style>
  .chat-container {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 56px);
    margin: -28px -32px;
    background: var(--c-bg);
  }

  /* Header */
  .chat-header {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    background: var(--c-surface);
    border-bottom: 1px solid var(--c-border, #e5e7eb);
    flex-shrink: 0;
  }
  .back-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 10px;
    font-size: 13px;
    color: var(--c-primary);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.15s;
  }
  .back-btn:hover {
    background: var(--c-primary-light);
  }
  .back-arrow {
    font-size: 16px;
  }
  .header-center {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }
  .header-avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--c-primary);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 700;
  }
  .header-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--c-text);
    margin: 0;
  }
  .header-spacer {
    width: 70px; /* balance the back button */
  }

  /* Messages area */
  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .chat-loading, .chat-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--c-text-muted);
    gap: 8px;
  }
  .chat-empty small {
    font-size: 12px;
  }

  /* Date separator */
  .date-separator {
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 12px 0;
  }
  .date-separator span {
    font-size: 11px;
    color: var(--c-text-muted);
    background: var(--c-bg);
    padding: 2px 12px;
    border-radius: 10px;
    border: 1px solid var(--c-border, #e5e7eb);
  }

  /* Message rows */
  .msg-row {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    max-width: 80%;
  }
  .msg-row.mine {
    align-self: flex-end;
    flex-direction: row-reverse;
  }
  .msg-row.theirs {
    align-self: flex-start;
  }

  .msg-avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: #9ca3af;
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .msg-bubble {
    padding: 10px 14px;
    border-radius: 16px;
    max-width: 100%;
    word-wrap: break-word;
    word-break: break-word;
  }
  .msg-row.mine .msg-bubble {
    background: var(--c-primary);
    color: #fff;
    border-bottom-right-radius: 4px;
  }
  .msg-row.theirs .msg-bubble {
    background: var(--c-surface);
    color: var(--c-text);
    border-bottom-left-radius: 4px;
    border: 1px solid var(--c-border, #e5e7eb);
  }

  .msg-body {
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .msg-meta {
    display: flex;
    justify-content: flex-end;
    margin-top: 4px;
  }
  .msg-time {
    font-size: 10px;
    opacity: 0.7;
  }

  /* Input area */
  .chat-input-area {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    background: var(--c-surface);
    border-top: 1px solid var(--c-border, #e5e7eb);
    flex-shrink: 0;
  }
  .chat-input {
    flex: 1;
    padding: 10px 14px;
    font-size: 13px;
    border: 1px solid var(--c-border, #e5e7eb);
    border-radius: 20px;
    background: var(--c-bg);
    color: var(--c-text);
    outline: none;
    transition: border-color 0.15s;
  }
  .chat-input:focus {
    border-color: var(--c-primary);
  }
  .chat-input:disabled {
    opacity: 0.6;
  }

  .send-btn {
    padding: 10px 20px;
    font-size: 13px;
    font-weight: 600;
    color: #fff;
    background: var(--c-primary);
    border: none;
    border-radius: 20px;
    cursor: pointer;
    transition: opacity 0.15s;
    flex-shrink: 0;
  }
  .send-btn:hover:not(:disabled) {
    opacity: 0.9;
  }
  .send-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* 이전 메시지 로딩 */
  .loading-older {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px 0;
    color: var(--c-text-muted);
    font-size: 12px;
  }
  .spinner-sm {
    width: 14px;
    height: 14px;
    border: 2px solid var(--c-border, #e5e7eb);
    border-top-color: var(--c-primary);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  .load-older-btn {
    display: block;
    margin: 0 auto 8px;
    padding: 6px 16px;
    font-size: 12px;
    color: var(--c-primary);
    background: var(--c-primary-light);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .load-older-btn:hover {
    background: var(--c-primary);
    color: #fff;
  }
</style>
