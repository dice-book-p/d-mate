<script>
  let { title = "", children } = $props();
</script>

<section class="card">
  {#if title}
    <h3 class="card-title">{title}</h3>
  {/if}
  <div class="card-body">
    {@render children()}
  </div>
</section>

<style>
  .card {
    background: var(--c-surface);
    border: 1px solid var(--c-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
  }
  .card-title {
    font-size: 14px;
    font-weight: 600;
    padding: 16px 20px 0;
    color: var(--c-text);
  }
  .card-body {
    padding: 16px 20px 20px;
  }
</style>
