use miniconf::Miniconf;

#[derive(Clone, Copy, Debug, Default, Miniconf)]
pub struct Settings {
    /// LED0 state.
    ///
    /// # Path
    /// `led`
    ///
    /// # Value
    /// "true" or "false".
    pub led: bool,
}
