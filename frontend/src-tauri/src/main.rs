// Always use the Windows GUI subsystem so launching the .exe (debug or release)
// never allocates a console window. Closing that console would kill the process.
// Logs go through tauri-plugin-log / tracing, not a visible console.
#![windows_subsystem = "windows"]

fn main() {
  app_lib::run();
}
