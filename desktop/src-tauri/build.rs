fn main() {
  // bundled-sqlcipher → OpenSSL → precisa de user32 e crypt32 no Windows.
  // Mesmo motivo do build.rs do backend. Se der LNK1120 ao rodar `tauri dev`,
  // é aqui que se ajusta.
  if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=crypt32");
  }
  tauri_build::build()
}
