/**
 * 두 객체의 지정 필드를 비교하여 변경 여부를 반환
 * @param {object} original - 원본 설정
 * @param {object} current - 현재 값
 * @param {string[]} fields - 비교할 필드 목록
 * @returns {boolean} 변경되었으면 true
 */
export function isDirty(original, current, fields) {
  if (!original || !current) return false;
  for (const key of fields) {
    const a = original[key];
    const b = current[key];
    if (String(a ?? "") !== String(b ?? "")) return true;
  }
  return false;
}

/**
 * 원본 스냅샷을 생성 (deep copy 필요 없음 — 단순 값만)
 */
export function snapshot(obj, fields) {
  const snap = {};
  for (const key of fields) {
    snap[key] = obj[key];
  }
  return snap;
}
