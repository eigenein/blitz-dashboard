pub trait InspectErr {
    fn stable_inspect_err(self, inspect: fn(&anyhow::Error)) -> Self;
}

impl<T> InspectErr for anyhow::Result<T> {
    fn stable_inspect_err(self, inspect: fn(&anyhow::Error)) -> Self {
        if let Err(error) = &self {
            inspect(error);
        }
        self
    }
}
