pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(feature = "optional_transitive_dep")]
pub fn num() -> usize {
    optional_transitive_dep::num()
}

#[cfg(not(feature = "optional_transitive_dep"))]
pub fn num() -> usize {
    42
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
