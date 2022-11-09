use tracing::instrument;
use tracing;

#[instrument]
pub fn add(left: usize, right: usize) -> usize {
    tracing::debug!("adding {} and {}", left, right);
    left + right
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
