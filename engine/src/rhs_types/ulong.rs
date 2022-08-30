use crate::{
    lex::{expect, span, take_while, Lex, LexErrorKind, LexResult},
    strict_partial_ord::StrictPartialOrd,
};
use serde::Serialize;
use std::ops::RangeInclusive;

fn lex_digits(input: &str) -> LexResult<'_, &str> {
    // Lex any supported digits (up to radix 16) for better error locations.
    take_while(input, "digit", |c| c.is_digit(16))
}

fn parse_number<'i>((input, rest): (&'i str, &'i str), radix: u32) -> LexResult<'_, u64> {
    match u64::from_str_radix(input, radix) {
        Ok(res) => Ok((res, rest)),
        Err(err) => Err((LexErrorKind::ParseInt { err, radix }, input)),
    }
}

impl<'i> Lex<'i> for u64 {
    fn lex(input: &str) -> LexResult<'_, Self> {
        if let Ok(input) = expect(input, "0x") {
            parse_number(lex_digits(input)?, 16)
        } else if input.starts_with('0') {
            // not using `expect` because we want to include `0` too
            parse_number(lex_digits(input)?, 8)
        } else {
            let without_neg = match expect(input, "-") {
                Ok(input) => input,
                Err(_) => input,
            };

            let (_, rest) = lex_digits(without_neg)?;

            parse_number((span(input, rest), rest), 10)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct UlongRange(RangeInclusive<u64>);

impl From<u64> for UlongRange {
    fn from(i: u64) -> Self {
        UlongRange(i..=i)
    }
}

impl From<RangeInclusive<u64>> for UlongRange {
    fn from(r: RangeInclusive<u64>) -> Self {
        UlongRange(r)
    }
}

impl<'i> Lex<'i> for UlongRange {
    fn lex(input: &str) -> LexResult<'_, Self> {
        let initial_input = input;
        let (first, input) = u64::lex(input)?;
        let (last, input) = if let Ok(input) = expect(input, "..") {
            u64::lex(input)?
        } else {
            (first, input)
        };
        if last < first {
            return Err((
                LexErrorKind::IncompatibleRangeBounds,
                span(initial_input, input),
            ));
        }
        Ok(((first..=last).into(), input))
    }
}

impl From<UlongRange> for RangeInclusive<u64> {
    fn from(range: UlongRange) -> Self {
        range.0
    }
}

impl StrictPartialOrd for u64 {}

#[test]
fn test() {
    use std::str::FromStr;

    assert_ok!(u64::lex("0"), 0u64, "");
    assert_ok!(u64::lex("0-"), 0u64, "-");
    assert_ok!(u64::lex("0x1f5+"), 501u64, "+");
    assert_ok!(u64::lex("0123;"), 83u64, ";");
    assert_ok!(u64::lex("78!"), 78u64, "!");
    assert_ok!(u64::lex("0xefg"), 239u64, "g");
    assert_ok!(u64::lex("2147483648!"), 2147483648u64, "!");
    assert_ok!(u64::lex("0xffffffffffffffff!"), 0xffffffffffffffff, "!");
    assert_err!(
        u64::lex("-12-"),
        LexErrorKind::ParseInt {
            err: u64::from_str("-2147483649").unwrap_err(),
            radix: 10
        },
        "-12"
    );
    assert_err!(
        u64::lex("-2147483649!"),
        LexErrorKind::ParseInt {
            err: u64::from_str("-2147483649").unwrap_err(),
            radix: 10
        },
        "-2147483649"
    );
    assert_err!(
        u64::lex("10fex"),
        LexErrorKind::ParseInt {
            err: u64::from_str("10fe").unwrap_err(),
            radix: 10
        },
        "10fe"
    );
    assert_ok!(UlongRange::lex("78!"), 78u64.into(), "!");
    assert_ok!(UlongRange::lex("0..10"), (0u64..=10u64).into());
    assert_ok!(UlongRange::lex("0123..0xefg"), (83u64..=239u64).into(), "g");
    assert_err!(
        UlongRange::lex("10..0"),
        LexErrorKind::IncompatibleRangeBounds,
        "10..0"
    );
}
