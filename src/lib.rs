//! Minimal SMTP envelope address parsing.
//!
//! This crate intentionally does **not** implement RFC 5322 mailboxes.
//! It accepts only addr-spec forms used in SMTP commands:
//!
//! - `local@domain`
//! - `<local@domain>`
//! - `<>` (null reverse-path)
//!
//! Display names, comments, and header syntax are rejected.

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Addr {
    /// The local "john" in "john@doe.com" or "<john@doe.com>"
    pub local: String,
    /// The domain following the '@' symbol
    pub domain: String,
}

impl Addr {
    pub fn parse_envelope(s: &str) -> Result<Self, AddrError> {
        // Accept "<a@b>" or "a@b". Reject display-name mailbox forms.
        let mut t = s.trim();
        if t == "<>" {
            // "<>" is explicitly allowed as a null address
            return Ok(Addr {
                local: String::new(),
                domain: String::new(),
            });
        }
        let mut is_bracketed = false;
        if let Some(lstrip) = t.strip_prefix("<") {
            is_bracketed = true;
            t = lstrip;
        }
        if let Some(rstrip) = t.strip_suffix(">") {
            if !is_bracketed {
                // There wasn't an opening <
                return Err(AddrError::InvalidBrackets)
            }
            t = rstrip
        } else {
            if is_bracketed {
                // There wasn't a closing >
                return Err(AddrError::InvalidBrackets)
            }
        }
        if t.chars().any(|c| c.is_whitespace()) {
            // Whitespace indicates separation between a display name and address
            return Err(AddrError::Whitespace)
        }
        if t.contains('<') || t.contains('>') {
            // Nested or stray angle brackets are invalid after stripping
            return Err(AddrError::InvalidBrackets);
        }
        if t.is_empty() {
            return Err(AddrError::Empty);
        }
        let (local, domain) = t
            .split_once("@")
            .ok_or(AddrError::MissingAt)?;
        if local.is_empty() || domain.is_empty() {
            return Err(AddrError::Empty);
        }
        if domain.contains("@") {
            return Err(AddrError::InvalidCharacter)
        }
        Ok(Addr {
            local: local.to_string(),
            domain: domain.to_ascii_lowercase(),
        })
    }

    pub fn to_addr_spec(&self) -> String {
        let mut s = String::with_capacity(self.local.len() + 1 + self.domain.len());
        s.push_str(&self.local);
        s.push('@');
        s.push_str(&self.domain);
        s
    }

    pub fn is_null(&self) -> bool {
        self.local.is_empty() && self.domain.is_empty()
    }

    pub fn to_bracketed(&self) -> String {
        let a = self.to_addr_spec();
        let mut s = String::with_capacity(a.len() + 2);
        s.push('<');
        s.push_str(&a);
        s.push('>');
        s
    }

    pub fn with_domain(&self, domain: impl Into<String>) -> Addr {
        Addr {
            local: self.local.clone(),
            domain: domain.into(),
        }
    }
}



impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "<>")
        } else {
            write!(f, "{}@{}", self.local, self.domain)
        }
    }
}

impl FromStr for Addr {
    type Err = AddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Addr::parse_envelope(s)
    }
}

#[derive(Debug, Error)]
/// Possible reasons for address parsing failure
pub enum AddrError {
    #[error("address was empty")]
    /// Evaluated as "" after trimming. Excludes null addr "<>"
    Empty,
    #[error("address did not contain '@'")]
    /// Address did not include '@'
    MissingAt,
    #[error("address contained malformed brackets")]
    /// Address was missing opening/closing bracket if one was present, or too many were in email
    InvalidBrackets,
    #[error("address contained invalid character(s)")]
    /// Address included a forbidden character, like a 2nd '@'
    InvalidCharacter,
    #[error("address contained whitespace or a display name")]
    /// Address included whitespace in email, which usually means it was a RFC 5322 address
    Whitespace,
}

#[cfg(test)]
mod tests {
    use super::Addr;

    #[test]
    fn parses_plain_address_and_normalizes_domain() {
        let addr = Addr::parse_envelope("LOCAL@Example.COM").expect("address should parse");
        assert_eq!(addr.local, "LOCAL");
        assert_eq!(addr.domain, "example.com");
        assert_eq!(addr.to_addr_spec(), "LOCAL@example.com");
    }

    #[test]
    fn parses_bracketed_address_and_formats_output() {
        let addr = Addr::parse_envelope("  <bounce+tag@Sub.Example.com>  ")
            .expect("address should parse");
        assert_eq!(addr.local, "bounce+tag");
        assert_eq!(addr.domain, "sub.example.com");
        assert_eq!(addr.to_bracketed(), "<bounce+tag@sub.example.com>");
    }

    #[test]
    fn parses_bracket_null_address() {
        let null_addr = Addr::parse_envelope(" <> ")
            .expect("null `<>` address should parse");
        assert!(null_addr.is_null(), "addr is null")
    }

    #[test]
    fn trims_whitespace_and_preserves_local_case() {
        let addr = Addr::parse_envelope("\n <MixedCase@Example.ORG>\t")
            .expect("address should parse");
        assert_eq!(addr.local, "MixedCase");
        assert_eq!(addr.domain, "example.org");
        assert_eq!(addr.to_addr_spec(), "MixedCase@example.org");
    }

    #[test]
    fn rejects_invalid_or_display_name_forms() {
        let cases = [
            "",
            "missingatsign",
            "local@",
            "@domain",
            "Name <alice@example.com>",
            "<alice@example.com",
            "alice@example.com>",
            "<@example.com>",
        ];

        for case in cases {
            assert!(Addr::parse_envelope(case).is_err(), "{case:?} should be rejected");
        }
    }

    #[test]
    fn rejects_addr_specs_with_whitespace() {
        let cases = [
            "alice smith@example.com",
            "<alice smith@example.com>",
            "alice@exa mple.com",
        ];

        for case in cases {
            assert!(Addr::parse_envelope(case).is_err(), "{case:?} should be rejected");
        }
    }

    #[test]
    fn rejects_addresses_with_multiple_ats_or_nested_brackets() {
        let cases = [
            "alice@@example.com",
            "alice@example@com",
            "<<alice@example.com>>",
            "<ali<ce>@example.com>",
        ];

        for case in cases {
            assert!(Addr::parse_envelope(case).is_err(), "{case:?} should be rejected");
        }
    }

    #[test]
    fn supports_internationalized_components() {
        let addr = Addr::parse_envelope("δοκιμή@MÜNICH.Example.COM")
            .expect("unicode address should parse");
        assert_eq!(addr.local, "δοκιμή");
        assert_eq!(addr.domain, "mÜnich.example.com");
        assert_eq!(addr.to_addr_spec(), "δοκιμή@mÜnich.example.com");
    }

    #[test]
    fn parses_unicode_domain_and_local_part() {
        let addr = Addr::parse_envelope("álïcé@例え.テスト").expect("unicode address should parse");
        assert_eq!(addr.local, "álïcé");
        assert_eq!(addr.domain, "例え.テスト");
        assert_eq!(addr.to_addr_spec(), "álïcé@例え.テスト");
    }
}