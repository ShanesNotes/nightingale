# Sourced by 10-rust.sh and 40-mutants.sh, so the sweep builds in the environment 10-rust tested.
# Cargo takes the first flag source it finds, so an empty RUSTFLAGS shuts out a host config's
# `--cap-lints allow`, which would silently disable forbid. RUSTC_BOOTSTRAP would admit #![feature] and
# with it the unstable float methods the bans probe does not classify; UPDATE_EXPECT would rewrite a trace.
export RUSTFLAGS="" RUSTDOCFLAGS=""
unset CARGO_ENCODED_RUSTFLAGS CARGO_ENCODED_RUSTDOCFLAGS CLIPPY_CONF_DIR RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER RUSTC_BOOTSTRAP \
  UPDATE_EXPECT
