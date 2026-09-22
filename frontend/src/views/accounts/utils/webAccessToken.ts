const ACCESS_TOKEN_KEYS = new Set(['accessToken', 'access_token'])
const SESSION_TOKEN_KEYS = new Set(['sessionToken', 'session_token'])
const MAX_TOKEN_BYTES = 16 * 1024

/**
 * Extract the OAuth access token from the common ChatGPT/Codex credential shapes.
 * `sessionToken` is deliberately ignored: it is an encrypted browser session,
 * not the bearer token accepted by the reset-credit endpoint.
 */
export function extractWebAccessToken(input: string): string {
  const value = input.trim()
  if (!value)
    throw new Error('请输入网页 Access Token 或粘贴完整的 /api/auth/session JSON')

  const parsed = parseJson(value)
  const extracted = parsed === undefined
    ? extractLabeledToken(value) ?? normalizeToken(value)
    : findAccessToken(parsed) ?? (typeof parsed === 'string' ? normalizeToken(parsed) : undefined)

  if (!extracted)
    throw new Error('未找到 accessToken。请粘贴裸 Token、Bearer Token 或完整的 /api/auth/session JSON')

  return validateWebAccessToken(extracted)
}

function parseJson(value: string): unknown {
  const source = stripCodeFence(value)
  try {
    return JSON.parse(source) as unknown
  }
  catch {
    return undefined
  }
}

function stripCodeFence(value: string): string {
  return value
    .replace(/^\s*```(?:json)?\s*/, '')
    .replace(/\s*```\s*$/, '')
    .trim()
}

function findAccessToken(value: unknown, depth = 0): string | undefined {
  if (depth > 12 || value === null || typeof value !== 'object')
    return undefined

  if (Array.isArray(value)) {
    for (const item of value) {
      const token = findAccessToken(item, depth + 1)
      if (token)
        return token
    }
    return undefined
  }

  const record = value as Record<string, unknown>
  for (const key of ACCESS_TOKEN_KEYS) {
    const token = record[key]
    if (typeof token === 'string' && token.trim())
      return normalizeToken(token)
  }

  for (const [key, child] of Object.entries(record)) {
    if (SESSION_TOKEN_KEYS.has(key))
      continue
    const token = findAccessToken(child, depth + 1)
    if (token)
      return token
  }
  return undefined
}

function extractLabeledToken(value: string): string | undefined {
  const match = value.match(/(?:access[_-]?token|authorization)\s*[:=]\s*(?:Bearer\s+)?["']?([^\s"'`,}]+)/i)
  return match?.[1] ? normalizeToken(match[1]) : undefined
}

function normalizeToken(value: string): string {
  let token = value.trim()
  token = token.replace(/^Bearer\s+/i, '').trim()
  if ((token.startsWith('\"') && token.endsWith('\"')) || (token.startsWith('\'') && token.endsWith('\'')))
    token = token.slice(1, -1).trim()
  return token
}

function validateWebAccessToken(value: string): string {
  const token = normalizeToken(value)
  if (!token || token.length > MAX_TOKEN_BYTES || [...token].some(character => /\p{Cc}/u.test(character)))
    throw new Error('网页 Access Token 格式无效，请粘贴 accessToken 字段而不是 sessionToken')
  if (token.startsWith('at-'))
    throw new Error('检测到 at- Personal Access Token，请粘贴网页响应中的 accessToken 字段')
  if (isEncryptedSessionToken(token))
    throw new Error('检测到 sessionToken，请粘贴网页响应中的 accessToken 字段')
  return token
}

function isEncryptedSessionToken(value: string): boolean {
  const segments = value.split('.')
  if (segments.length !== 5)
    return false
  try {
    const header = JSON.parse(decodeBase64Url(segments[0])) as { alg?: string, enc?: string }
    return header.alg === 'dir' && header.enc === 'A256GCM'
  }
  catch {
    return false
  }
}

function decodeBase64Url(value: string): string {
  const normalized = value.replace(/-/g, '+').replace(/_/g, '/')
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=')
  const binary = atob(padded)
  return Array.from(binary, character => String.fromCodePoint(character.charCodeAt(0))).join('')
}
