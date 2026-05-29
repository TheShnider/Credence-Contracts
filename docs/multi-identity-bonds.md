# Multi-Identity Bonds Architecture

This document covers the architectural layout and keying mechanics that enable the `credence_bond` smart contract to handle multiple, separate identity stakes within a singular instance safely. This transition eliminates the bottleneck of deploying one contract instance per identity, maximizing gas efficiency and simplifying synchronization overhead.

---

## Storage Partitioning Strategy

To isolate identity assets while avoiding collisions, `credence_bond` leverages a custom Soroban dynamic key formatting layout stored directly inside the **Persistent Storage** tier. This design guarantees that data keys scale unboundedly without bleeding states into neighboring profiles.

### The Keying Scheme

Bonds are explicitly compartmentalized using an identity parameter mapping structure. The storage serialization model maps keys under a discrete Rust enum payload structure:

```rust
#[derive(Clone)]
#[meta_type]
pub enum DataKey {
    Admin,
    Token,
    Bond(Address),               // Identity Dynamic Keying Target
    Nonce(Address),             // Replay Prevention Tracker per Identity
    SubjectAttestations(Address) // Attestation Graph Routing
}
```
