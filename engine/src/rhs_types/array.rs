use crate::{
    lex::{expect, skip_space, Lex, LexErrorKind, LexResult, LexWith},
    lhs_types::Array,
    prelude::*,
    strict_partial_ord::StrictPartialOrd,
    types::{GetType, RhsValue, Type},
};
use alloc::borrow::Borrow;
use core::cmp::Ordering;
use serde::Serialize;

/// Array literal for RHS values, e.g., {1 2 3}
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize)]
pub struct RhsArray {
    elements: Vec<RhsValue>,
    element_type: Type,
}

impl RhsArray {
    /// Creates a new RHS array with the given elements and element type
    pub fn new(elements: Vec<RhsValue>, element_type: Type) -> Self {
        Self {
            elements,
            element_type,
        }
    }

    /// Returns the elements of the array
    pub fn elements(&self) -> &[RhsValue] {
        &self.elements
    }

    /// Returns the element type of the array
    pub fn element_type(&self) -> Type {
        self.element_type
    }
}

impl PartialEq<RhsArray> for Array<'_> {
    fn eq(&self, other: &RhsArray) -> bool {
        if self.value_type() != other.element_type {
            return false;
        }
        if self.len() != other.elements.len() {
            return false;
        }

        // Compare elements - this is an approximation since we'd need proper conversion
        let left = self.as_slice();
        let right = &other.elements;
        left.iter().zip(right.iter()).all(|(l, r)| l == r)
    }
}

impl PartialOrd<RhsArray> for Array<'_> {
    fn partial_cmp(&self, other: &RhsArray) -> Option<Ordering> {
        // Compare types first
        if self.value_type() != other.element_type {
            return None; // Different types are incomparable
        }

        // Compare lengths
        match self.len().cmp(&other.elements.len()) {
            Ordering::Equal => {}
            other_ordering => return Some(other_ordering),
        }

        let left = self.as_slice();
        let right = &other.elements;
        for (l, r) in left.iter().zip(right.iter()) {
            match l.strict_partial_cmp(r) {
                Some(Ordering::Equal) => {}
                Some(other_ordering) => return Some(other_ordering),
                None => return None,
            }
        }
        Some(Ordering::Equal)
    }
}

impl StrictPartialOrd<RhsArray> for Array<'_> {}

impl Lex<'_> for RhsArray {
    fn lex(input: &str) -> LexResult<'_, Self> {
        let mut input = expect(input, "{")?;
        let mut elements = Vec::new();
        let mut element_type = None;

        loop {
            input = skip_space(input);
            if let Ok(rest) = expect(input, "}") {
                let final_type = element_type.unwrap_or(Type::Int); // Default to Int for empty arrays
                return Ok((RhsArray::new(elements, final_type), rest));
            }

            // Try to parse different types of literals
            if let Ok((literal, rest)) = RhsValue::lex_with(input, Type::Int) {
                elements.push(literal);
                element_type = Some(Type::Int);
                input = rest;
            } else if let Ok((literal, rest)) = RhsValue::lex_with(input, Type::Bytes) {
                elements.push(literal);
                if element_type.is_none() {
                    element_type = Some(Type::Bytes);
                }
                input = rest;
            } else if let Ok((literal, rest)) = RhsValue::lex_with(input, Type::Ip) {
                elements.push(literal);
                if element_type.is_none() {
                    element_type = Some(Type::Ip);
                }
                input = rest;
            } else {
                return Err((LexErrorKind::EOF, input));
            }
        }
    }
}

impl GetType for RhsArray {
    fn get_type(&self) -> Type {
        Type::Array(self.element_type.into())
    }
}

impl GetType for Vec<RhsArray> {
    fn get_type(&self) -> Type {
        // For RhsValues (used in field in {value1, value2} operations)
        // We'll use the type of the first element or default to Int
        self.first()
            .map(|arr| arr.get_type())
            .unwrap_or(Type::Array(Type::Int.into()))
    }
}

/// [Uninhabited / empty type](https://doc.rust-lang.org/nomicon/exotic-sizes.html#empty-types)
/// for `array` with traits we need for RHS values.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize)]
pub enum UninhabitedArray {}

impl<'a> Borrow<Array<'a>> for UninhabitedArray {
    fn borrow(&self) -> &Array<'a> {
        match *self {}
    }
}

impl PartialEq<UninhabitedArray> for Array<'_> {
    fn eq(&self, other: &UninhabitedArray) -> bool {
        match *other {}
    }
}

impl PartialOrd<UninhabitedArray> for Array<'_> {
    fn partial_cmp(&self, other: &UninhabitedArray) -> Option<Ordering> {
        match *other {}
    }
}

// impl StrictPartialOrd<UninhabitedArray> for Array<'_> {}

impl Lex<'_> for UninhabitedArray {
    fn lex(_input: &str) -> LexResult<'_, Self> {
        unreachable!()
    }
}

impl GetType for UninhabitedArray {
    fn get_type(&self) -> Type {
        unreachable!()
    }
}

impl GetType for Vec<UninhabitedArray> {
    fn get_type(&self) -> Type {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lhs_types::Array,
        strict_partial_ord::StrictPartialOrd,
        types::{RhsValue, Type},
    };

    #[test]
    fn test_strict_partial_ord() {
        // Create test arrays with different types and content
        let int_array_1 = RhsArray::new(
            vec![RhsValue::Int(1), RhsValue::Int(2), RhsValue::Int(3)],
            Type::Int,
        );

        let int_array_2 = RhsArray::new(
            vec![RhsValue::Int(1), RhsValue::Int(2), RhsValue::Int(4)],
            Type::Int,
        );

        let int_array_empty = RhsArray::new(vec![], Type::Int);

        let bytes_array = RhsArray::new(
            vec![
                RhsValue::Bytes("hello".as_bytes().to_vec().into()),
                RhsValue::Bytes("world".as_bytes().to_vec().into()),
            ],
            Type::Bytes,
        );

        // Create corresponding LHS arrays for comparison
        let lhs_int_array_1 = Array::from_iter([1i64, 2i64, 3i64]);
        let lhs_int_array_2 = Array::from_iter([1i64, 2i64, 4i64]);
        let lhs_int_array_empty = Array::new(Type::Int);
        let lhs_bytes_array = Array::from_iter(["hello", "world"]);

        // Test same type arrays - should use normal partial_cmp
        assert_eq!(
            lhs_int_array_1.strict_partial_cmp(&int_array_1),
            lhs_int_array_1.partial_cmp(&int_array_1)
        );

        assert_eq!(
            lhs_int_array_1.strict_partial_cmp(&int_array_2),
            lhs_int_array_1.partial_cmp(&int_array_2)
        );

        // Test empty arrays
        assert_eq!(
            lhs_int_array_empty.strict_partial_cmp(&int_array_empty),
            lhs_int_array_empty.partial_cmp(&int_array_empty)
        );

        // Test different type arrays - might be incomparable
        // Note: The current implementation of PartialEq/PartialOrd for Array vs RhsArray
        // returns false/None for different types, so strict_partial_cmp should do the same
        assert_eq!(
            lhs_bytes_array.strict_partial_cmp(&int_array_1),
            lhs_bytes_array.partial_cmp(&int_array_1)
        );

        // Test that strict_partial_cmp is consistent with partial_cmp
        let test_cases = [
            (&lhs_int_array_1, &int_array_1),
            (&lhs_int_array_2, &int_array_2),
            (&lhs_int_array_empty, &int_array_empty),
            (&lhs_bytes_array, &bytes_array),
        ];

        for (lhs_array, rhs_array) in test_cases {
            assert_eq!(
                lhs_array.strict_partial_cmp(rhs_array),
                lhs_array.partial_cmp(rhs_array),
                "strict_partial_cmp should be consistent with partial_cmp for same-type arrays"
            );
        }

        // Test cross-type comparisons
        let cross_type_cases = [
            (&lhs_int_array_1, &bytes_array),
            (&lhs_bytes_array, &int_array_1),
            (&lhs_int_array_empty, &bytes_array),
        ];

        for (lhs_array, rhs_array) in cross_type_cases {
            assert_eq!(
                lhs_array.strict_partial_cmp(rhs_array),
                lhs_array.partial_cmp(rhs_array),
                "strict_partial_cmp should be consistent with partial_cmp for cross-type arrays"
            );
        }
    }

    #[test]
    fn test_rhs_array_creation_and_getters() {
        let elements = vec![RhsValue::Int(42), RhsValue::Int(100)];
        let array = RhsArray::new(elements.clone(), Type::Int);

        assert_eq!(array.elements(), &elements);
        assert_eq!(array.element_type(), Type::Int);
        assert_eq!(array.get_type(), Type::Array(Type::Int.into()));
    }

    #[test]
    fn test_rhs_array_partial_eq_with_lhs_array() {
        let rhs_array = RhsArray::new(vec![RhsValue::Int(1), RhsValue::Int(2)], Type::Int);
        let lhs_array = Array::from_iter([1i64, 2i64]);
        let rhs_array_diff = RhsArray::new(vec![RhsValue::Int(1), RhsValue::Int(3)], Type::Int);

        assert!(lhs_array.eq(&rhs_array));
        assert!(!lhs_array.eq(&rhs_array_diff));

        // Different types should not be equal
        let rhs_bytes_array = RhsArray::new(
            vec![RhsValue::Bytes("test".as_bytes().to_vec().into())],
            Type::Bytes,
        );
        assert!(!lhs_array.eq(&rhs_bytes_array));

        // Different lengths should not be equal
        let rhs_different_length = RhsArray::new(
            vec![RhsValue::Int(1), RhsValue::Int(2), RhsValue::Int(3)],
            Type::Int,
        );
        assert!(!lhs_array.eq(&rhs_different_length));
    }
}
