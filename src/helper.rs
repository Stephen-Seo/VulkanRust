#[macro_export]
macro_rules! cleanup_func {
    (func: $cleanup_fn:expr, name: $name:ident, hold_name: $hold_name:ident) => {
        struct $name <T: Fn() -> ()> {
            func: T,
        }
        impl<T> Drop for $name <T> where T: Fn() -> () {
            fn drop(&mut self) {
                (self.func)();
            }
        }
        impl<T> $name <T> where T: Fn() -> () {
            fn new(func: T) -> Self where T: Fn() -> () {
                Self {
                    func,
                }
            }
        }
        $hold_name = $name::new($cleanup_fn);
    };
    (func: $cleanup_fn:expr, name: $name:ident, hold_name: $hold_name:ident, $(var_pair: $orig_var:expr, $new_var:ident),*) => {
        {
            $(let $new_var = $orig_var;)*
            cleanup_func!(func: $cleanup_fn, name: $name, hold_name: $hold_name);
        }
    }
}
