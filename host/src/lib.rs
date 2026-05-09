//! Host crate runs with the full std lib and can import `shared`. Unit tests
//! could live here as well as probably a simulator of some fashion.

pub use shared::add;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_can_call_shared_add() {
        assert_eq!(add(40, 2), 42);
    }
}
