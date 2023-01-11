#[macro_export]
macro_rules! demite {
    ($x:expr) => {
        if log::log_enabled!(log::Level::Debug) {
            dbg!($x);
        }
    };
}

