// Plumbs the `LX_NIGHTLY` env var through to a compile-time
// `LX_NIGHTLY_MARKER` rustc-env directive, used by the `VERSION`
// const in `src/options/parser.rs`.  When set non-empty (the
// nightly CI workflow sets `LX_NIGHTLY=1`), the marker becomes
// ` [nightly]` and is appended to `--version` output; otherwise
// it's the empty string and `--version` is unchanged.

fn main() {
    println!("cargo:rerun-if-env-changed=LX_NIGHTLY");

    let marker = match std::env::var("LX_NIGHTLY") {
        Ok(v) if !v.is_empty() => " [nightly]",
        _ => "",
    };
    println!("cargo:rustc-env=LX_NIGHTLY_MARKER={marker}");
}
