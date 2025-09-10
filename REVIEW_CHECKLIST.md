# Security Review Checklist for SECURITY-AGENT-1

## Critical Security Patterns to Monitor

### 1. SQL Injection Vulnerabilities
- **Pattern**: Direct string concatenation in SQL queries
- **Look for**: `format!()`, `&format!()`, string concatenation with `+` in SQL contexts
- **Safe patterns**: SQLx with `$1, $2` placeholders, `query_as!()` macros
- **Risk Level**: CRITICAL

### 2. Authentication/Authorization Issues
- **Pattern**: Missing authentication checks, hardcoded credentials, weak token validation
- **Look for**: 
  - Endpoints without auth middleware
  - Hardcoded API keys, passwords, secrets
  - JWT without proper validation
  - Role-based access not enforced
- **Risk Level**: CRITICAL

### 3. Input Validation Failures
- **Pattern**: Unvalidated user input, missing sanitization
- **Look for**: 
  - Direct use of user input without validation
  - Missing `#[validate(...)]` attributes
  - `unwrap()` on user-provided data
  - Regex without bounds checking
- **Risk Level**: HIGH

### 4. Unsafe Rust Code
- **Pattern**: Usage of `unsafe` blocks, raw pointers
- **Look for**: 
  - `unsafe { ... }` blocks
  - `panic!()` in production code
  - `unwrap()` that could panic on user input
  - Buffer overflow potential
- **Risk Level**: HIGH

### 5. Secrets Management Issues
- **Pattern**: Hardcoded secrets, logging sensitive data
- **Look for**: 
  - API keys, passwords in source code
  - Logging of sensitive fields
  - Secrets in configuration files
  - Missing environment variable usage
- **Risk Level**: CRITICAL

### 6. CSRF Protection
- **Pattern**: State-changing operations without CSRF protection
- **Look for**: 
  - POST/PUT/DELETE endpoints without CSRF tokens
  - Missing SameSite cookie attributes
  - Unprotected admin operations
- **Risk Level**: MEDIUM

### 7. Rate Limiting Bypass
- **Pattern**: Missing or weak rate limiting
- **Look for**: 
  - Endpoints without rate limiting middleware
  - Rate limit bypass techniques
  - Missing IP-based protection
- **Risk Level**: MEDIUM

### 8. Information Disclosure
- **Pattern**: Verbose error messages, debug info in production
- **Look for**: 
  - Database errors returned to client
  - Stack traces in responses
  - Internal paths/structure exposed
  - Debug logs with sensitive data
- **Risk Level**: MEDIUM

### 9. Insecure Deserialization
- **Pattern**: Unsafe deserialization of untrusted data
- **Look for**: 
  - `serde` without proper validation
  - Custom deserializers without bounds
  - Binary deserialization of user input
- **Risk Level**: HIGH

### 10. Session Management Issues
- **Pattern**: Weak session handling, insecure cookies
- **Look for**: 
  - Sessions without proper expiration
  - Missing HttpOnly/Secure flags
  - Session fixation vulnerabilities
- **Risk Level**: HIGH

## Risk Assessment Matrix

### CRITICAL (Immediate Action Required)
- SQL Injection
- Authentication Bypass
- Hardcoded Secrets
- Remote Code Execution

### HIGH (Review within 24 hours)
- Input Validation Issues
- Unsafe Rust Code
- Insecure Deserialization
- Session Management Flaws

### MEDIUM (Review within 1 week)
- CSRF Vulnerabilities
- Rate Limiting Issues
- Information Disclosure
- Weak Access Controls

### LOW (Review during next sprint)
- Minor information leaks
- Non-critical logging issues
- Performance-related security concerns

## File-Specific Security Contexts

### `/src/handlers/auth.rs`
- Focus on authentication logic, token validation, password handling
- Check for timing attacks, credential enumeration

### `/src/handlers/ingest.rs`
- Focus on input validation, data sanitization, batch processing
- Check for injection attacks, data corruption

### `/src/middleware/auth.rs`
- Focus on authentication middleware, token extraction, validation
- Check for bypass conditions, weak validation

### `/src/middleware/rate_limit.rs`
- Focus on rate limiting logic, IP handling, bypass prevention
- Check for race conditions, limit circumvention

### `/src/services/auth.rs`
- Focus on core authentication services, user management
- Check for privilege escalation, weak crypto usage

## Common Rust Security Anti-Patterns

1. **Panic in Production**: `unwrap()`, `expect()` on user input
2. **Unsafe Operations**: Raw pointer dereference, transmute abuse
3. **Integer Overflow**: Unchecked arithmetic operations
4. **Resource Exhaustion**: Unbounded loops, memory allocation
5. **Race Conditions**: Shared mutable state without proper locking

## Security-Specific Code Review Questions

1. Is user input properly validated and sanitized?
2. Are authentication checks present and correctly implemented?
3. Are authorization checks properly enforced?
4. Are sensitive operations properly logged (without exposing secrets)?
5. Are error messages safe for production (no internal info leak)?
6. Are all database queries parameterized?
7. Are secrets properly externalized?
8. Is rate limiting properly implemented?
9. Are security headers properly set?
10. Is the code free from timing attack vulnerabilities?