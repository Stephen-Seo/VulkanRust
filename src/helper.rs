pub struct Cleanup<T: Fn()> {
    func: T,
}

impl<T> Drop for Cleanup<T>
where
    T: Fn(),
{
    fn drop(&mut self) {
        (self.func)();
    }
}

impl<T> Cleanup<T>
where
    T: Fn(),
{
    pub fn new(func: T) -> Self {
        Self { func }
    }
}

#[macro_export]
macro_rules! cleanup_func {
    (func: $cleanup_fn:expr,
     hold_name: $hold_name:ident,
     $(var_pair: $orig_var:expr, $new_var:ident),*) => {
        $(let $new_var = $orig_var;)*

        $hold_name = crate::helper::Cleanup::new($cleanup_fn);
    }
}
