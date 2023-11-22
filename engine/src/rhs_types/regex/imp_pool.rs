use lazy_static::lazy_static;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

pub use regex::Error;

/// Wrapper around [`regex::bytes::Regex`]
#[derive(Clone)]
pub struct Regex(Arc<regex::bytes::Regex>);

lazy_static! {
    static ref REGEX_POOL: Mutex<HashSet<Regex>> = Mutex::new(HashSet::new());
}

impl Drop for Regex {
    fn drop(&mut self) {
        // check whether this is the last strong reference to the regex, and
        // avoid deadlock by making sure to drop the last cached regex only
        // after we've dropped the lock on the pool.
        let cached_regex = if Arc::strong_count(&self.0) == 2 && Arc::weak_count(&self.0) == 0 {
            let mut pool = REGEX_POOL.lock().unwrap();
            pool.take(self.as_str())
        } else {
            None
        };

        // now we can safely drop the regex, as there's no deadlock
        drop(cached_regex);
    }
}

impl FromStr for Regex {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let mut pool = REGEX_POOL.lock().unwrap();
        if let Some(regex) = pool.get(s) {
            return Ok(regex.clone());
        }

        let regex = Self(Arc::new(
            ::regex::bytes::RegexBuilder::new(s)
                .unicode(false)
                .build()?,
        ));

        pool.insert(regex.clone());
        Ok(regex)
    }
}

impl Regex {
    /// Returns true if and only if the regex matches the string given.
    pub fn is_match(&self, text: &[u8]) -> bool {
        self.0.is_match(text)
    }

    /// Returns the original string of this regex.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for Regex {
    fn borrow(&self) -> &str {
        self.0.as_str()
    }
}
