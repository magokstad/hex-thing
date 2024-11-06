pub fn parse_num(input: &str) -> Result<usize, std::num::ParseIntError> {
    if input.starts_with("0x") {
        usize::from_str_radix(&input[2..], 16)
    } else {
        usize::from_str_radix(input, 10)
    }
}

pub trait ApplyIf: Sized {
    fn apply_if<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self;
}

impl<T> ApplyIf for T {
    fn apply_if<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }
}
