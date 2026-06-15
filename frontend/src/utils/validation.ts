// Frontend input validation utilities

export function validateDomain(domain: string): { valid: boolean; error?: string } {
  if (!domain || domain.trim().length === 0) {
    return { valid: false, error: 'Domain cannot be empty' }
  }

  if (domain.length > 253) {
    return { valid: false, error: 'Domain too long (max 253 characters)' }
  }

  // Basic domain pattern: alphanumeric, dots, hyphens
  const domainPattern = /^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?(\.[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?)*$/i
  if (!domainPattern.test(domain)) {
    return { valid: false, error: 'Invalid domain format' }
  }

  return { valid: true }
}

export function validatePort(port: number | string): { valid: boolean; error?: string } {
  const portNum = typeof port === 'string' ? parseInt(port, 10) : port

  if (isNaN(portNum)) {
    return { valid: false, error: 'Port must be a number' }
  }

  if (portNum < 1 || portNum > 65535) {
    return { valid: false, error: 'Port must be between 1 and 65535' }
  }

  // Warn about privileged ports
  if (portNum < 1024) {
    return { valid: true, error: 'Warning: Port below 1024 may require admin privileges' }
  }

  return { valid: true }
}

export function validateDatabaseName(name: string): { valid: boolean; error?: string } {
  if (!name || name.trim().length === 0) {
    return { valid: false, error: 'Database name cannot be empty' }
  }

  if (name.length > 64) {
    return { valid: false, error: 'Database name too long (max 64 characters)' }
  }

  // MySQL identifier rules: alphanumeric and underscore only
  const namePattern = /^[a-zA-Z0-9_]+$/
  if (!namePattern.test(name)) {
    return { valid: false, error: 'Database name can only contain letters, numbers, and underscores' }
  }

  // Cannot start with a number
  if (/^\d/.test(name)) {
    return { valid: false, error: 'Database name cannot start with a number' }
  }

  return { valid: true }
}

export function validatePath(path: string): { valid: boolean; error?: string } {
  if (!path || path.trim().length === 0) {
    return { valid: false, error: 'Path cannot be empty' }
  }

  // Windows path validation - basic check
  if (/[<>"|?*]/.test(path)) {
    return { valid: false, error: 'Path contains invalid characters' }
  }

  return { valid: true }
}

export function validatePassword(password: string, minLength = 6): { valid: boolean; error?: string } {
  if (!password || password.length === 0) {
    return { valid: false, error: 'Password cannot be empty' }
  }

  if (password.length < minLength) {
    return { valid: false, error: `Password must be at least ${minLength} characters` }
  }

  return { valid: true }
}
