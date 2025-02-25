use crate::App;

pub trait Plugin {
    fn build(&self, app: &mut App);
}
