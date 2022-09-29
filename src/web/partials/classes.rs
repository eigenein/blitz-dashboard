use maud::PreEscaped;

macro_rules! class {
    ($ident:ident, $value:literal) => {
        pub const $ident: PreEscaped<&str> = PreEscaped($value);
    };
}

class!(HAS_BACKGROUND_DANGER_LIGHT, "has-background-danger-light");
class!(HAS_BACKGROUND_SUCCESS_LIGHT, "has-background-success-light");
