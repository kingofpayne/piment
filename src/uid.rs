use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

/// Identifies uniquely an object.
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Uid(u64);

impl Uid {
    /// Creates a new random Uid.
    pub fn new() -> Self {
        Self(rand::rng().random::<u64>())
    }
}

impl Default for Uid {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for Uid {
    type Err = std::num::ParseIntError;

    /// Parses a [Uid] from its hexadecimal string representation (the format produced by
    /// [Display]). A leading `0x` prefix is accepted.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let s = s
            .strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(s);
        u64::from_str_radix(s, 16).map(Uid)
    }
}

impl Display for Uid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// This macro helps wrapping [Uid] into a new type, giving type safety on UIDs.
/// This prevents mixing UIDs of objects of different purpose.
#[macro_export]
macro_rules! wrap_uid {
    ($name:ident) => {
        #[derive(
            Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uid);

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl From<Uid> for $name {
            fn from(uid: Uid) -> Self {
                $name(uid)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
