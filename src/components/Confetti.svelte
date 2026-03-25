<script>
  import { confettiState } from "../lib/easter.js";

  let active = $state(false);
  let particles = $state([]);

  confettiState.subscribe((v) => {
    if (v) {
      active = true;
      particles = Array.from({ length: 60 }, (_, i) => ({
        id: i,
        x: Math.random() * 100,
        delay: Math.random() * 0.5,
        duration: 1.5 + Math.random() * 1.5,
        size: 6 + Math.random() * 6,
        color: ["#3b82f6", "#f59e0b", "#10b981", "#ef4444", "#8b5cf6", "#ec4899"][i % 6],
        rotation: Math.random() * 360,
        drift: -30 + Math.random() * 60,
      }));
      setTimeout(() => {
        active = false;
        confettiState.set(false);
      }, 3500);
    }
  });
</script>

{#if active}
  <div class="confetti-container">
    {#each particles as p}
      <div
        class="confetti-piece"
        style="
          left: {p.x}%;
          animation-delay: {p.delay}s;
          animation-duration: {p.duration}s;
          --drift: {p.drift}px;
          --rotation: {p.rotation}deg;
        "
      >
        <div
          class="confetti-inner"
          style="
            width: {p.size}px;
            height: {p.size * 0.6}px;
            background: {p.color};
          "
        ></div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .confetti-container {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 99999;
    overflow: hidden;
  }
  .confetti-piece {
    position: absolute;
    top: -20px;
    animation: confetti-fall linear forwards;
  }
  .confetti-inner {
    border-radius: 2px;
    animation: confetti-spin linear infinite;
    animation-duration: 0.8s;
  }
  @keyframes confetti-fall {
    0% {
      transform: translateY(0) translateX(0) rotate(0deg);
      opacity: 1;
    }
    80% {
      opacity: 1;
    }
    100% {
      transform: translateY(100vh) translateX(var(--drift)) rotate(var(--rotation));
      opacity: 0;
    }
  }
  @keyframes confetti-spin {
    from { transform: rotateX(0) rotateY(0); }
    to { transform: rotateX(360deg) rotateY(180deg); }
  }
</style>
