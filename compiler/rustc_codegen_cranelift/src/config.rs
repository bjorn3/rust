/// Configuration of cg_clif as passed in through `-Cllvm-args`.
#[derive(Clone, Debug)]
pub struct BackendConfig {
    /// When JIT executing should the lazy JIT mode be used.
    ///
    /// Defaults to false. Can be set using `-Cllvm-args=jit-lazy`.
    pub lazy_jit: bool,
}

impl BackendConfig {
    /// Parse the configuration passed in using `-Cllvm-args`.
    pub fn from_opts(opts: &[String]) -> Result<Self, String> {
        let mut config = BackendConfig { lazy_jit: false };

        for opt in opts {
            if opt.starts_with("-import-instr-limit") {
                // Silently ignore -import-instr-limit. It is set by rust's build system even when
                // testing cg_clif.
                continue;
            }
            match &**opt {
                "jit-lazy" => config.lazy_jit = true,
                _ => return Err(format!("Unknown option `{}`", opt)),
            }
        }

        Ok(config)
    }
}
