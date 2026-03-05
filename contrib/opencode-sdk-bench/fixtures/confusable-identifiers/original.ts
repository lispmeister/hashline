function normalizeUser(value: string): string {
  return value.trim().toLowerCase();
}

function normalizeUѕer(value: string): string {
  return value.trim();
}

export function compareUser(a: string, b: string): boolean {
  return normalizeUser(a) === normalizeUser(b);
}
