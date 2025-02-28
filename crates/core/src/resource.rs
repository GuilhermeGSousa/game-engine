pub trait Resource: 'static + Sized {
    fn name() -> String;
}
