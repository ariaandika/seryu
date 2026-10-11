pub trait Global {
    fn global<'a>() -> &'a mut Self;
}
