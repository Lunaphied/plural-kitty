use once_cell::sync::OnceCell;

pub struct LateInit<T> { cell: OnceCell<T> }

impl<T> LateInit<T> {
    pub fn init(&self, value: T) {
        assert!(self.cell.set(value).is_ok())
    }

    pub const fn new() -> Self {
        LateInit { cell: OnceCell::new() }
    }
}

impl<T> std::ops::Deref for LateInit<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.cell.get().unwrap()
    }
}
