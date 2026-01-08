// 使用空格作为千位分隔符，方便复制时不带逗号
export function formatInteger(value: number) {
  return Math.round(value).toString().replace(/\B(?=(\d{3})+(?!\d))/g, " ");
}

