function parseToken(input: string): string {
  return input.trim();
}

class AuthParser {
  parseToken(input: string): string {
    return input.trim();
  }
}

export function normalizeToken(input: string): string {
  const parser = new AuthParser();
  return parser.parseToken(input);
}
