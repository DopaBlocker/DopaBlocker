// O bundled-sqlcipher puxa OpenSSL (via vcpkg, no Windows). O libcrypto.lib
// chama APIs Win32 (MessageBoxW, Cert*Store*, GetProcessWindowStation...) que
// vivem em `user32.lib` e `crypt32.lib` — e esses não são linkados por padrão.
// Declarar aqui resolve o LNK2019/LNK1120 sem precisar mexer em RUSTFLAGS
// global ou .cargo/config.toml.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=crypt32");
    }
}
