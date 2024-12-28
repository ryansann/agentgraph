use std::fmt::Debug;

pub trait GraphState: Debug + Send + Sync + Clone + 'static {
    type Update;

    fn apply(&mut self, update: Self::Update);

    // Default implementation for applying multiple updates
    fn apply_many<I: IntoIterator<Item = Self::Update>>(&mut self, updates: I) {
        for update in updates {
            self.apply(update);
        }
    }
}
