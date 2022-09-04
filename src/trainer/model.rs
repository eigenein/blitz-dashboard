use crate::prelude::*;
use crate::trainer::Regression;

#[derive(Default)]
pub struct Model {
    pub regressions: AHashMap<
        wargaming::Realm,
        AHashMap<wargaming::TankId, AHashMap<wargaming::TankId, Regression>>,
    >,
}
