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
