<script>
  import Card from "../components/Card.svelte";
  import { onMount, onDestroy } from "svelte";
  import {
    getMessages, markMessageRead, getUnreadCount, getMqttStatus,
    getConversations, getContacts,
  } from "../lib/api.js";
  import { unreadCount, showToast, navigateTo, chatParams } from "../lib/stores.js";
  import { listen } from "@tauri-apps/api/event";

  let activeTab = $state("notices");
  let messages = $state([]);
  let conversations = $state([]);
  let contacts = $state([]);
  let loading = $state(true);
  let mqttConnected = $state(false);
  let unlisten = null;

  const typeLabel = { notice: "전체 공지", notify: "개인 알림", dm: "메시지", unknown: "알림" };
  const typeIcon = { notice: "📢", notify: "🔔", dm: "💬", unknown: "🔔" };

  onMount(async () => {
    await loadMessages();
    await checkMqtt();
    loading = false;

    unlisten = await listen("mqtt:message", (event) => {
      const msg = event.payload;
      messages = [
        { ...msg, is_read: 0, received_at: new Date().toISOString() },
        ...messages,
      ];
      refreshUnread();
      // 대화 탭이 활성이면 대화 목록도 갱신
      if (activeTab === "conversations") {
        loadConversations();
      }
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function loadMessages() {
    try {
      const data = await getMessages(null, 100);
      messages = Array.isArray(data) ? data : [];
    } catch (e) {
      console.error("Failed to load messages:", e);
      messages = [];
    }
    await refreshUnread();
  }

  async function loadConversations() {
    try {
      const data = await getConversations();
      if (data?.error) {
        conversations = [];
      } else {
        conversations = Array.isArray(data) ? data : (data?.conversations || []);
      }
    } catch (e) {
      console.error("Failed to load conversations:", e);
      conversations = [];
    }
  }

  async function loadContacts() {
    try {
      const data = await getContacts();
      if (data?.error) {
        contacts = [];
      } else {
        contacts = Array.isArray(data) ? data : (data?.contacts || []);
      }
    } catch (e) {
      console.error("Failed to load contacts:", e);
      contacts = [];
    }
  }

  async function checkMqtt() {
    try {
      const s = await getMqttStatus();
      mqttConnected = s.connected;
    } catch (e) {
      mqttConnected = false;
    }
  }

  async function refreshUnread() {
    try {
      const r = await getUnreadCount();
      unreadCount.set(r.count || 0);
    } catch (e) { /* ignore */ }
  }

  async function markRead(msg) {
    if (msg.is_read) return;
    try {
      await markMessageRead(msg.id);
      msg.is_read = 1;
      messages = [...messages];
      await refreshUnread();
    } catch (e) { /* ignore */ }
  }

  function switchTab(tab) {
    activeTab = tab;
    if (tab === "conversations" && conversations.length === 0) {
      loadConversations();
    }
    if (tab === "contacts" && contacts.length === 0) {
      loadContacts();
    }
  }

  function openChat(conv) {
    chatParams.set({
      conv_id: conv.conversation_id || conv.id,
      target_code: conv.target_code || conv.other_code || "",
      target_name: conv.target_name || conv.other_name || "대화",
    });
    navigateTo("chat");
  }

  function startDm(contact) {
    chatParams.set({
      conv_id: contact.conversation_id || "",
      target_code: contact.code || contact.member_code || "",
      target_name: contact.name || contact.nickname || "사용자",
    });
    navigateTo("chat");
  }

  function formatTime(dateStr) {
    if (!dateStr) return "";
    try {
      const d = new Date(dateStr.replace(" ", "T") + (dateStr.includes("Z") ? "" : "Z"));
      const now = new Date();
      const diff = now - d;
      if (diff < 60000) return "방금";
      if (diff < 3600000) return `${Math.floor(diff / 60000)}분 전`;
      if (diff < 86400000) return `${Math.floor(diff / 3600000)}시간 전`;
      return d.toLocaleDateString("ko-KR", { month: "short", day: "numeric" });
    } catch {
      return dateStr;
    }
  }

  // 공지 + 개인알림 필터 (DM 제외)
  let noticeMessages = $derived(messages.filter((m) => m.type !== "dm"));
</script>

<div class="page">
  <div class="page-header">
    <h2 class="page-title">메시지</h2>
    <div class="mqtt-status" class:online={mqttConnected}>
      <span class="status-dot"></span>
      {mqttConnected ? "실시간 연결" : "오프라인"}
    </div>
  </div>

  <!-- Tabs -->
  <div class="tabs">
    <button class="tab" class:active={activeTab === "notices"} onclick={() => switchTab("notices")}>
      공지
    </button>
    <button class="tab" class:active={activeTab === "conversations"} onclick={() => switchTab("conversations")}>
      대화
    </button>
    <button class="tab" class:active={activeTab === "contacts"} onclick={() => switchTab("contacts")}>
      연락처
    </button>
  </div>

  {#if loading}
    <div class="loading-state"><div class="spinner"></div><p>로딩 중...</p></div>

  <!-- 공지 탭 -->
  {:else if activeTab === "notices"}
    {#if noticeMessages.length === 0}
      <div class="empty-state">
        <span class="empty-icon">📢</span>
        <p>수신된 공지가 없습니다.</p>
        <small>공지사항이나 알림이 수신되면 여기에 표시됩니다.</small>
      </div>
    {:else}
      <div class="message-list">
        {#each noticeMessages as msg}
          <button
            class="message-item"
            class:unread={!msg.is_read}
            onclick={() => markRead(msg)}
          >
            <span class="msg-type-icon">{typeIcon[msg.type] || "🔔"}</span>
            <div class="msg-content">
              <div class="msg-top">
                <span class="msg-type-label type-{msg.type}">{typeLabel[msg.type] || "알림"}</span>
                {#if msg.sender_name}
                  <span class="msg-sender">{msg.sender_name}</span>
                {/if}
                <span class="msg-time">{formatTime(msg.created_at)}</span>
              </div>
              <div class="msg-title">{msg.title || "(제목 없음)"}</div>
              {#if msg.body}
                <div class="msg-body">{msg.body}</div>
              {/if}
            </div>
            {#if !msg.is_read}
              <span class="unread-dot"></span>
            {/if}
          </button>
        {/each}
      </div>
    {/if}

  <!-- 대화 탭 -->
  {:else if activeTab === "conversations"}
    {#if conversations.length === 0}
      <div class="empty-state">
        <span class="empty-icon">💬</span>
        <p>대화가 없습니다.</p>
        <small>연락처 탭에서 새 대화를 시작할 수 있습니다.</small>
      </div>
    {:else}
      <div class="message-list">
        {#each conversations as conv}
          <button class="message-item conv-item" onclick={() => openChat(conv)}>
            <span class="conv-avatar">
              {(conv.target_name || conv.other_name || "?").charAt(0)}
            </span>
            <div class="msg-content">
              <div class="msg-top">
                <span class="conv-name">{conv.target_name || conv.other_name || "알 수 없음"}</span>
                <span class="msg-time">{formatTime(conv.last_message_at || conv.updated_at)}</span>
              </div>
              <div class="msg-body">{conv.last_message || conv.last_body || ""}</div>
            </div>
            {#if (conv.unread_count || 0) > 0}
              <span class="conv-unread">{conv.unread_count}</span>
            {/if}
          </button>
        {/each}
      </div>
    {/if}

  <!-- 연락처 탭 -->
  {:else if activeTab === "contacts"}
    {#if contacts.length === 0}
      <div class="empty-state">
        <span class="empty-icon">👥</span>
        <p>연락처가 없습니다.</p>
        <small>Desk 서버에 등록된 멤버가 표시됩니다.</small>
      </div>
    {:else}
      <div class="message-list">
        {#each contacts as contact}
          <div class="message-item contact-item">
            <span class="conv-avatar">
              {(contact.name || contact.nickname || "?").charAt(0)}
            </span>
            <div class="msg-content">
              <div class="contact-info">
                <span class="contact-name">{contact.name || contact.nickname || "알 수 없음"}</span>
                {#if contact.status === "online"}
                  <span class="contact-online">온라인</span>
                {:else}
                  <span class="contact-offline">오프라인</span>
                {/if}
              </div>
              {#if contact.role}
                <div class="msg-body">{contact.role}</div>
              {/if}
            </div>
            <button class="dm-start-btn" onclick={() => startDm(contact)}>
              메시지
            </button>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .mqtt-status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--c-text-muted);
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #9ca3af;
  }
  .mqtt-status.online .status-dot {
    background: var(--c-success);
  }
  .mqtt-status.online {
    color: var(--c-success);
  }

  /* Tabs */
  .tabs {
    display: flex;
    gap: 0;
    margin-bottom: 16px;
    border-bottom: 2px solid var(--c-border, #e5e7eb);
  }
  .tab {
    padding: 10px 20px;
    font-size: 13px;
    font-weight: 600;
    color: var(--c-text-muted);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }
  .tab:hover {
    color: var(--c-text);
  }
  .tab.active {
    color: var(--c-primary);
    border-bottom-color: var(--c-primary);
  }

  .empty-state {
    text-align: center;
    padding: 60px 20px;
    color: var(--c-text-muted);
  }
  .empty-icon {
    font-size: 40px;
    display: block;
    margin-bottom: 12px;
  }
  .empty-state p {
    font-size: 14px;
    margin-bottom: 4px;
  }
  .empty-state small {
    font-size: 12px;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 40px 0;
    color: var(--c-text-muted);
  }

  .message-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .message-item {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 14px 16px;
    background: var(--c-surface);
    border-radius: var(--radius-sm);
    text-align: left;
    width: 100%;
    transition: background 0.15s;
    position: relative;
    cursor: pointer;
  }
  .message-item:hover {
    background: var(--c-surface-hover);
  }
  .message-item.unread {
    background: var(--c-primary-light);
  }
  .message-item.unread:hover {
    background: #dbeafe;
  }

  .msg-type-icon {
    font-size: 20px;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .msg-content {
    flex: 1;
    min-width: 0;
  }

  .msg-top {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .msg-type-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--c-primary);
    background: var(--c-primary-light);
    padding: 1px 6px;
    border-radius: 3px;
  }
  .msg-type-label.type-notice {
    color: #b45309;
    background: #fef3c7;
  }
  .msg-type-label.type-notify {
    color: #6d28d9;
    background: #ede9fe;
  }
  .msg-sender {
    font-size: 12px;
    color: var(--c-text-secondary);
  }
  .msg-time {
    font-size: 11px;
    color: var(--c-text-muted);
    margin-left: auto;
    flex-shrink: 0;
  }

  .msg-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--c-text);
    margin-bottom: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .msg-body {
    font-size: 12px;
    color: var(--c-text-secondary);
    line-height: 1.5;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .unread-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--c-primary);
    flex-shrink: 0;
    margin-top: 8px;
  }

  /* Conversation items */
  .conv-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--c-primary);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .conv-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--c-text);
  }
  .conv-unread {
    background: #ef4444;
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 10px;
    min-width: 20px;
    text-align: center;
    flex-shrink: 0;
    align-self: center;
  }

  /* Contact items */
  .contact-item {
    cursor: default;
    align-items: center;
  }
  .contact-info {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 2px;
  }
  .contact-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--c-text);
  }
  .contact-online {
    font-size: 11px;
    color: var(--c-success);
    font-weight: 500;
  }
  .contact-offline {
    font-size: 11px;
    color: var(--c-text-muted);
  }
  .dm-start-btn {
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 600;
    color: var(--c-primary);
    background: var(--c-primary-light);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s, color 0.15s;
  }
  .dm-start-btn:hover {
    background: var(--c-primary);
    color: #fff;
  }
</style>
