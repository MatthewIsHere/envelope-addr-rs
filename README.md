# envelope-addr

Minimal parsing and formatting for **SMTP envelope addresses**.

This crate handles only what SMTP actually uses in commands like `MAIL FROM` and `RCPT TO`.
It intentionally does **not** implement RFC 5322 header mailboxes.

---

## What it supports

Accepted forms:

* `local@domain`
* `<local@domain>`
* `<>` (null reverse-path, used for bounces)

Behavior:

* Leading/trailing whitespace is trimmed
* Domain is ASCII-lowercased when possible
* Local part case is preserved
* Unicode local parts and domains are allowed (no IDNA conversion)

---

## What it rejects

* Display names (`Name <user@domain>`)
* Comments
* Quoted local parts
* Whitespace inside the address
* Header-only mailbox syntax

If it wouldn’t appear in an SMTP command, it doesn’t belong here.

---

## Example

```rust
use envelope_addr::Addr;

let addr = Addr::parse_envelope(" <User@Example.COM> ")?;
assert_eq!(addr.local, "User");
assert_eq!(addr.domain, "example.com");
assert_eq!(addr.to_addr_spec(), "User@example.com");
assert_eq!(addr.to_bracketed(), "<User@example.com>");
```

Null reverse-path:

```rust
let null = Addr::parse_envelope("<>")?;
assert!(null.is_null());
```

---

## Design goals

* Small, dependency-light
* Deterministic and fully unit-testable
* Suitable for MTAs, milters, SMTP proxies, and SRS implementations
* Clear separation between envelope parsing and higher-level mail logic

---

## Non-goals

* RFC 5322 compliance
* Header parsing
* Address normalization beyond SMTP needs

---

## License

Licensed under the MIT license

---

