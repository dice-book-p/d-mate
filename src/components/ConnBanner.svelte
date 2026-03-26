<script>
  import { navigateTo } from "../lib/stores.js";

  let { items = [] } = $props();
  // items: [{ message, linkText, linkPage, linkTab }]

  function go(item) {
    navigateTo("connection");
  }
</script>

{#if items.length > 0}
  <div class="conn-banners">
    {#each items as item}
      <div class="conn-banner">
        <span class="banner-icon">⚠</span>
        <span class="banner-text">{item.message}</span>
        <button class="banner-link" onclick={() => go(item)}>
          {item.linkText} →
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .conn-banners {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 20px;
  }
  .conn-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: #fffbeb;
    border: 1px solid #fde68a;
    border-radius: var(--radius-md);
    font-size: 13px;
    color: #92400e;
  }
  .banner-icon {
    font-size: 14px;
    flex-shrink: 0;
  }
  .banner-text {
    flex: 1;
  }
  .banner-link {
    background: none;
    border: none;
    color: var(--c-primary);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    padding: 0;
  }
  .banner-link:hover {
    text-decoration: underline;
  }
</style>
