#[macro_export]
/// Round a number up to a multiple greater than or equal to a specified numerical value of the number
macro_rules! ceil_to_multiple {
    ($value:expr, $multiple:expr) => {{
        let value = $value;
        let multiple = $multiple;

        if multiple == 0 || value % multiple == 0 {
            value
        } else {
            value + (multiple - (value % multiple))
        }
    }};
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn macro_ceil_to_multiple() {
        assert_eq!(ceil_to_multiple!(66, 10), 70);
        assert_eq!(ceil_to_multiple!(5, 8), 8);
        assert_eq!(ceil_to_multiple!(6, 0), 6);
        assert_eq!(ceil_to_multiple!(0, 4), 0);
    }
}
