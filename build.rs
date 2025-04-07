const COMMANDS: &[&str] = &[
   "create", "list", "get", "start", "cancel", "pause", "resume",
];

fn main() {
   tauri_plugin::Builder::new(COMMANDS).build();
}
