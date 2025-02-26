pub fn remove_suffix<'a>(s: &'a str, suffix: &str) -> &'a str {
   match s.strip_suffix(suffix) {
      Some(s) => s,
      None => s,
   }
}
