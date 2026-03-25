import { writable } from "svelte/store";

export const confettiState = writable(false);

const messages = [
  "오늘도 수고했어요! 💪",
  "D-Mate가 응원합니다! 🎉",
  "최고예요! 🌟",
  "화이팅! 🔥",
  "멋진 하루 보내세요! ✨",
];

export function triggerConfetti() {
  confettiState.set(true);
  return messages[Math.floor(Math.random() * messages.length)];
}

/**
 * 야근 감지: 근무시간 외(18시 이후 or 주말)이면 메시지 반환
 */
export function checkOvertime() {
  const now = new Date();
  const hour = now.getHours();
  const day = now.getDay(); // 0=일, 6=토

  if (day === 0 || day === 6) {
    return "주말인데 일하고 계시네요... 쉬세요! 🌴";
  }
  if (hour >= 21) {
    return "이 시간까지...? 제발 퇴근하세요 🌙";
  }
  if (hour >= 19) {
    return "퇴근 시간이 지났어요. 오늘은 여기까지! 🏠";
  }
  if (hour < 7) {
    return "새벽 근무시네요... 건강 챙기세요 ☕";
  }
  return null;
}
