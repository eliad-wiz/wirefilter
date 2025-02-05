use crate::{FilterParser, GenericRegexMatcher, RegexFormat};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use thiserror::Error;

pub use regex::Error as RegexError;

/// Gen Regex errors
#[derive(Debug, PartialEq, Error)]
pub enum Error {
    /// Error in case a given pattern is not supported by the custom searcher
    #[error("Failed to add pattern {pattern:?}")]
    UnsupportedPattern {
        /// The original regex pattern
        pattern: String,
    },

    /// Error in case custom matcher is not set and we are using the RegexPool
    #[error("Regex error")]
    SimpleRegexErr(#[from] RegexError),
}

/// Wrapper around [`regex::bytes::Regex`]
#[derive(Clone)]
pub struct SimpleRegex {
    compiled_regex: regex::bytes::Regex,
    format: RegexFormat,
}

impl SimpleRegex {
    /// Compiles a regular expression.
    pub fn new(
        pattern: &str,
        format: RegexFormat,
        parser: &FilterParser<'_>,
    ) -> Result<Self, RegexError> {
        ::regex::bytes::RegexBuilder::new(pattern)
            .unicode(false)
            .size_limit(parser.regex_compiled_size_limit)
            .dfa_size_limit(parser.regex_dfa_size_limit)
            .build()
            .map(|r| SimpleRegex {
                compiled_regex: r,
                format,
            })
    }

    /// Returns true if and only if the regex matches the string given.
    pub fn is_match(&self, text: &[u8]) -> bool {
        self.compiled_regex.is_match(text)
    }

    /// Returns the original string of this regex.
    pub fn as_str(&self) -> &str {
        self.compiled_regex.as_str()
    }

    /// Returns the format behind the regex
    pub fn format(&self) -> RegexFormat {
        self.format
    }
}

impl PartialEq for SimpleRegex {
    fn eq(&self, other: &SimpleRegex) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for SimpleRegex {}

impl Hash for SimpleRegex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl From<SimpleRegex> for regex::bytes::Regex {
    fn from(regex: SimpleRegex) -> Self {
        regex.compiled_regex
    }
}

impl Debug for SimpleRegex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Regex wrapper to custom matchers
#[derive(Clone)]
pub struct GenRegex {
    matcher: Arc<Box<dyn GenericRegexMatcher>>,
    format: RegexFormat,
}

/// Regex wrapper, in case the parser has a custom matcher we will
/// store Regex::Gen objects, in case the parser did not set a custom
/// matcher use the default matcher.
#[derive(Clone)]
pub enum Regex {
    /// Custom matcher type
    Gen(GenRegex),
    /// Default Regex matcher
    Simple(SimpleRegex),
}

impl Regex {
    /// Creates a new dummy regex.
    pub fn new(
        pattern: &str,
        format: RegexFormat,
        parser: &FilterParser<'_>,
    ) -> Result<Self, Error> {
        let Some(re_builder) = parser.gen_regex_builder.as_ref() else {
            let simple_re = SimpleRegex::new(pattern, format, parser)?;
            return Ok(Self::Simple(simple_re));
        };

        let Some(matcher) = re_builder.build_pattern(pattern) else {
            return Err(Error::UnsupportedPattern {
                pattern: pattern.to_string(),
            });
        };

        Ok(Self::Gen(GenRegex {
            matcher: matcher.into(),
            format,
        }))
    }

    /// Not implemented and will panic if called.
    pub fn is_match(&self, text: &[u8]) -> bool {
        match self {
            Self::Gen(r) => r.matcher.as_ref().is_match(text),
            Self::Simple(r) => r.is_match(text),
        }
    }

    /// Returns the original string of this dummy regex wrapper.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Gen(r) => r.matcher.as_ref().as_str(),
            Self::Simple(r) => r.as_str(),
        }
    }

    /// Returns the format behind the regex
    pub fn format(&self) -> RegexFormat {
        match self {
            Self::Gen(r) => r.format,
            Self::Simple(r) => r.format(),
        }
    }
}

#[test]
fn test_compiled_size_limit() {
    use crate::Scheme;

    let scheme = Scheme::default();

    const COMPILED_SIZE_LIMIT: usize = 1024 * 1024;
    let mut parser = FilterParser::new(&scheme);
    parser.regex_set_compiled_size_limit(COMPILED_SIZE_LIMIT);
    assert_eq!(
        SimpleRegex::new(".{4079,65535}", RegexFormat::Literal, &parser),
        Err(RegexError::CompiledTooBig(COMPILED_SIZE_LIMIT))
    );
}
