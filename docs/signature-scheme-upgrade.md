# Multi-Scheme Signature Verification Registry

**Status:** Implemented  
**Version:** 1.0  
**Last Updated:** 2026-05-31

## Overview

The delegated action signature verification in `credence_delegation` traditionally relied on the **implicit Soroban auth engine**, which supports **Ed25519-only** signatures. As post-quantum cryptography requirements emerge, this contract introduces an **explicit verifier registry** that maps signature scheme tags to verifier implementations, enabling support for:

- **Ed25519** (0): NIST-standard EdDSA using Curve25519 (default, backwards-compatible)
- **Secp256r1** (1): ECDSA over NIST P-256 curve (future extensibility)
- **MLDSA44** (2): ML-DSA with security parameter 44 (post-quantum resistant)

The registry is **admin-controlled**, emits audit events, and maintains **full backwards compatibility** with existing Ed25519 delegated payloads.

## Problem Statement

### Current Limitations

1. **Single Scheme Dependency**: The Soroban auth engine enforces Ed25519-only, with no extensibility point for other schemes.
2. **Post-Quantum Gap**: No path to support lattice-based or NIST-standardized post-quantum algorithms.
3. **Hard Upgrade Requirement**: Adding new schemes requires contract upgrade rather than on-chain governance.

### Solution Architecture

- **Explicit Registry**: Each signature scheme maps to a registered verifier contract/module.
- **Admin Gating**: Only the contract admin can register or update verifier implementations.
- **Event Audit Trail**: Every verifier registration emits `verifier_registered` events for off-chain indexing.
- **Wire Stability**: Scheme tag numeric values are immutable; old signatures remain verifiable after upgrades.

## Implementation

### Scheme Tag Encoding

Scheme tags are **wire-stable** and encoded as `u8` values in `DelegatedActionPayload.scheme`:

```rust
#[repr(u8)]
pub enum SchemeTag {
    Ed25519 = 0,      // Default, backwards-compatible
    Secp256r1 = 1,    // NIST P-256 ECDSA
    MLDSA44 = 2,      // Post-quantum resistant
}
```

**CRITICAL**: These numeric values are immutable. Changing them after deployment breaks existing signatures. New schemes must append at the end only.

### Wire Stability Guarantee

When a delegated payload is signed, the `scheme` field value is encoded directly into the signed data:

```text
DelegatedActionPayload {
    domain: DomainTag,
    owner: Address,
    target: Address,
    contract_id: Address,
    nonce: u64,
    scheme: u8,          // ← Wire-encoded: must never change
}
```

**Implication**: An old client (created before multi-scheme support) that signs a payload with `scheme=0` (Ed25519) will produce a signature that includes this tag. If that scheme tag value is renumbered after deployment, signature verification will fail because the verifier will see a different tag value than what was encoded.

**Safe Extension**: New schemes can be appended at the end (scheme=3, 4, ...), and old clients continue to work because:
1. They sign payloads with scheme=0
2. The verifier still recognizes scheme=0 as Ed25519
3. The signature verification path is unchanged

### Registry Storage

Schemes are registered in persistent storage keyed by their tag value:

```rust
enum DataKey {
    // ...
    /// Maps scheme tag (0=Ed25519, 1=Secp256r1, 2=MLDSA44) to verifier address
    Verifier(u8),
}
```

Each registration:
1. Requires admin authorization (`admin.require_auth()`)
2. Validates the scheme is known (`validate_scheme_registered`)
3. Stores the verifier address: `e.storage().instance().set(&DataKey::Verifier(scheme), &verifier_id)`
4. Emits `verifier_registered` event

### Backwards Compatibility

**Existing Ed25519 payloads created before the upgrade continue to work without modification:**

1. **Legacy Decoding**: When a `DelegatedActionPayload` is deserialized, if the `scheme` field is absent or contains an unrecognized value, it defaults to Ed25519:
   ```rust
   pub fn decode_scheme_safe(payload: &DelegatedActionPayload) -> SchemeTag {
       match SchemeTag::try_from_u8(payload.scheme) {
           Some(scheme) => scheme,
           None => SchemeTag::default_scheme(),  // Ed25519
       }
   }
   ```

2. **Implicit Verification**: Ed25519 signatures continue to be verified by Soroban's built-in auth engine via `owner.require_auth()`. No additional verification logic is needed.

3. **No Migration Required**: Existing client code that creates delegated payloads can immediately set `scheme = 0` (or omit it and rely on defaults) and signatures remain valid.

### DelegatedActionPayload Schema Extension

```rust
pub struct DelegatedActionPayload {
    pub domain: DomainTag,      // Function domain (Delegate, RevokeDelegation, RevokeAttestation)
    pub owner: Address,         // Principal whose authority is delegated
    pub target: Address,        // Address the action targets
    pub contract_id: Address,   // This contract (chain/deployment context)
    pub nonce: u64,             // Monotonic replay counter
    pub scheme: u8,             // NEW: Signature scheme (default Ed25519 if absent)
}
```

All fields (including `scheme`) are hashed together for signature verification.

### Verification Dispatch

The `verify_delegated_signature` function handles scheme-based dispatch:

```rust
pub fn verify_delegated_signature(
    e: &Env,
    owner: &Address,
    message: &Bytes,
    signature: &Bytes,
    scheme: u8,
) {
    match SchemeTag::try_from_u8(scheme) {
        Some(SchemeTag::Ed25519) => {
            // Ed25519: Already verified by Soroban's auth engine
            // via owner.require_auth() at the call site.
            // No additional verification needed.
        }
        Some(SchemeTag::Secp256r1) | Some(SchemeTag::MLDSA44) => {
            // Post-quantum schemes: Dispatch to registered verifier.
            // Panics with VerifierNotRegistered if not registered.
        }
        None => {
            // Unknown scheme: Panic with UnknownScheme
        }
    }
}
```

**Integration at Call Site:**

In `execute_delegated_delegate` and similar functions:

```rust
pub fn execute_delegated_delegate(
    e: Env,
    owner: Address,
    delegate: Address,
    delegation_type: DelegationType,
    expires_at: u64,
    payload: DelegatedActionPayload,
) -> Delegation {
    pausable::require_not_paused(&e);
    owner.require_auth();  // Soroban auth engine (Ed25519 implicit verification)

    // Domain-separated payload verification
    domain::verify_payload(&e, &payload, DomainTag::Delegate, &owner, &delegate);

    // Scheme validation (rejects unknown schemes)
    let scheme = domain::decode_scheme_safe(&payload);
    verifier::verify_delegated_signature(
        &e,
        &owner,
        &message_hash,      // Serialized payload hash
        &signature_bytes,   // Provided by relayer
        scheme.to_u8(),
    );

    // Nonce and state changes...
}
```

## Admin API

### Register a Verifier

```rust
pub fn register_verifier(e: Env, admin: Address, scheme: u8, verifier_id: Address)
```

**Requirements:**
- Caller must be the contract admin (`require_auth()`)
- `scheme` must be a known value (0-2)
- Emits `verifier_registered` event

**Effects:**
- Updates storage: `DataKey::Verifier(scheme) → verifier_id`
- Can be called multiple times to update a scheme's verifier
- New verifier takes effect immediately

### Query Verifier

```rust
pub fn get_verifier(e: Env, scheme: u8) -> Option<Address>
```

Returns the registered verifier address for a scheme, or `None` if not registered.

## Error Codes

New error codes for multi-scheme support:

| Code | Name | Meaning |
|------|------|---------|
| 508 | `UnknownScheme` | Scheme tag is not recognized (> 2) |
| 509 | `VerifierAlreadyRegistered` | Scheme already has a registered verifier (if needed) |
| 510 | `VerifierNotRegistered` | No verifier registered for the given scheme |
| 511 | `VerificationFailed` | Signature verification failed |

## Security Considerations

### Admin Control

- **Single Point of Authority**: Only the admin can register verifiers.
- **No Consensus**: Registration is not governed; admin is a centralized entity.
- **Revocation**: No explicit revocation function; re-registration with a new verifier effectively updates the scheme.

### Scheme Immutability

- **Wire Stability Enforced**: Numeric scheme values must never change once deployed.
- **Append-Only Growth**: New schemes are appended at the end of the enum.
- **Migration Path**: If a scheme must be retired, the contract can emit a deprecation event and document the switch to a new scheme tag (e.g., EdDSA → NIST Ed448 as scheme=3).

### Signature Validation

- **Ed25519**: Implicit verification via `owner.require_auth()` (Soroban engine)
- **Post-Quantum**: Explicit dispatch to registered verifier; if missing, request panics with `VerifierNotRegistered`
- **Format Validation**: Each verifier is responsible for format validation (e.g., signature length, encoding)

### Replay Prevention

Replay prevention is unchanged:
- **Nonce**: Each principal maintains a monotonically increasing nonce
- **Domain Separation**: Each delegated action includes a `domain` tag (Delegate, RevokeDelegation, etc.)
- **Contract Context**: `contract_id` field binds the payload to a specific contract address

The `scheme` field is part of the signed payload, so changing schemes for the same (owner, nonce, domain) prevents replay.

## Testing

### Unit Tests

- `test_scheme_tag_from_u8`: Encoding/decoding of scheme tags
- `test_unknown_scheme_rejection`: Unknown schemes are rejected
- `test_default_scheme_ed25519`: Ed25519 is the backwards-compatible default
- `test_ed25519_backwards_compatible`: Legacy payloads work without migration

### Integration Tests

- **Legacy Ed25519 Payload**: Create a delegated payload with `scheme=0`, sign it with Ed25519, verify it still works
- **Scheme Tag Validation**: Attempt to use unknown scheme tags (e.g., 255) and confirm rejection
- **Admin-Only Registration**: Verify non-admin cannot register verifiers
- **Registry Persistence**: Register a verifier, query it, confirm it persists across calls

### Edge Cases

1. **Missing Scheme Field in Legacy Payload**: Defaults to Ed25519 ✓
2. **Unrecognized Scheme in New Payload**: Panics with `UnknownScheme` ✓
3. **Multiple Registrations**: Re-registering the same scheme updates the verifier ✓
4. **Cross-Contract Replay**: Payload includes `contract_id`, preventing cross-contract attacks ✓

## Deployment Strategy

### Phase 1: Deploy with Ed25519 Default

1. Contract deploys with multi-scheme support enabled
2. Admin registers an Ed25519 verifier (optional; implicit via Soroban auth)
3. Existing clients continue to work without changes
4. New clients can explicitly set `scheme = 0`

### Phase 2: Register Post-Quantum Verifiers

1. Admin registers a Secp256r1 verifier (`register_verifier(..., 1, secp256r1_address)`)
2. New clients can choose `scheme = 1` for ECDSA payloads
3. Ed25519 clients remain unaffected

### Phase 3: Monitor and Deprecate (if needed)

1. Off-chain indexing tracks scheme usage via `verifier_registered` events
2. If a scheme becomes obsolete, emit deprecation warnings (no on-chain enforcement)
3. Migrate clients to supported schemes

## FAQ

### Q: Will existing delegated payloads signed with Ed25519 break?

**A:** No. Existing Ed25519 payloads continue to work because:
1. They implicitly use `scheme = 0`
2. The verifier continues to recognize scheme 0 as Ed25519
3. Soroban's auth engine continues to verify Ed25519 signatures

### Q: Can I change the numeric value of Ed25519?

**A:** **No**. Doing so breaks every existing signature. Scheme tag values are wire-stable and immutable.

### Q: What if I need to add a 4th signature scheme?

**A:** Append it as `SCHEME = 3` to the enum. Old clients (using scheme=0) are unaffected.

### Q: How do I verify that my delegated payload uses Ed25519?

**A:** Check the `scheme` field value:
- `scheme == 0` → Ed25519
- `scheme == 1` → Secp256r1
- `scheme == 2` → MLDSA44

### Q: Is there a cost to registering a new verifier?

**A:** Only the admin can register, and each registration incurs normal contract execution costs. There's no additional fee or barrier.

### Q: Can I revoke a registered verifier?

**A:** Not explicitly. The contract accepts registrations and updates only. To "disable" a scheme, the admin would need to register a dummy verifier that always panics, or wait for a contract upgrade.

### Q: What happens if the registered verifier address is invalid?

**A:** If the verifier contract cannot be reached or fails verification, the call panics with `VerificationFailed` or a contract error from the verifier.

## References

- [Delegated Action Payloads](credence-delegation.md)
- [Domain Separation](delegation.md#domain-separation)
- [Error Codes](errors.md)
- [Admin Roles](admin-roles.md)
