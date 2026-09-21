use std::path::{Component, Path, PathBuf};

/// The directories a download may be written to.
///
/// Held as managed state and checked in the command layer, so one rule in Rust bounds
/// desktop, Android and iOS alike. Several rather than one because destinations need
/// not share a root: on iOS the sandbox separates `Documents` from `Library`.
#[derive(Debug)]
pub(crate) struct DownloadScope {
   roots: Vec<PathBuf>,
}

impl DownloadScope {
   /// Creates a scope bounded by `roots`, each of which must be absolute.
   ///
   /// Roots are normalized, not canonicalized, to match what [`check`](Self::check)
   /// does to the caller's path. Resolving one side only would reject `/var` against
   /// a root resolved to `/private/var`.
   pub(crate) fn new<I, P>(roots: I) -> crate::Result<Self>
   where
      I: IntoIterator<Item = P>,
      P: Into<PathBuf>,
   {
      let roots: Vec<PathBuf> = roots.into_iter().map(Into::into).collect();

      // No root admits nothing, which is unusable rather than merely narrow.
      if roots.is_empty() {
         return Err(reject("at least one download directory is required"));
      }

      // A relative root follows the working directory, and an empty one is not
      // absolute either, so one check covers both.
      for root in &roots {
         if !root.is_absolute() {
            return Err(reject("download directory must be absolute"));
         }
      }

      Ok(Self {
         roots: roots.iter().map(|root| normalize(root)).collect(),
      })
   }

   /// Checks that `path`, as received from the webview, is inside the scope.
   ///
   /// Structural validation runs first so a malformed path keeps the message every
   /// platform already sends for it, rather than being reported as out of scope.
   pub(crate) fn check(&self, path: &str) -> crate::Result<()> {
      download_manager::validate_path(path)?;

      let normalized = normalize(Path::new(path));

      if !self.roots.iter().any(|root| normalized.starts_with(root)) {
         return Err(reject("path must be inside a download directory"));
      }

      Ok(())
   }
}

/// Builds the error every rejection here reports.
///
/// The conversion is an identity on desktop and a real one on mobile, so one
/// implementation reports identically on both.
#[allow(clippy::useless_conversion)]
fn reject(message: &str) -> crate::Error {
   download_manager::Error::Path(message.to_string()).into()
}

/// Resolves `.` and `..` in a path without touching the file system.
///
/// The target of a download does not exist yet, so [`std::fs::canonicalize`] cannot be
/// used on it. A `..` at the root is dropped, so no run of them escapes above it, and
/// a symbolic link pointing out of the scope is not resolved.
fn normalize(path: &Path) -> PathBuf {
   let mut normalized = PathBuf::new();

   for component in path.components() {
      match component {
         Component::CurDir => {}
         Component::ParentDir => {
            normalized.pop();
         }
         component => normalized.push(component),
      }
   }

   normalized
}

#[cfg(test)]
mod tests {
   use super::*;

   fn scope() -> DownloadScope {
      DownloadScope::new(["/data/downloads"]).unwrap()
   }

   #[test]
   fn test_path_inside_the_scope_is_accepted() {
      assert!(scope().check("/data/downloads/file.zip").is_ok());
      assert!(scope().check("/data/downloads/nested/dir/file.zip").is_ok());
   }

   #[test]
   fn test_path_outside_the_scope_is_rejected() {
      assert!(scope().check("/etc/passwd").is_err());
      assert!(scope().check("/data/other/file.zip").is_err());
   }

   #[test]
   fn test_parent_traversal_out_of_the_scope_is_rejected() {
      // Resolved before the comparison, so a path is judged by where it lands.
      assert!(scope().check("/data/downloads/../../etc/passwd").is_err());
      assert!(
         scope()
            .check("/data/downloads/sub/../../other/f.zip")
            .is_err()
      );
   }

   #[test]
   fn test_parent_traversal_inside_the_scope_is_accepted() {
      // `..` is not itself suspicious if the path still lands inside the scope.
      assert!(scope().check("/data/downloads/sub/../file.zip").is_ok());
      assert!(scope().check("/data/downloads/./file.zip").is_ok());
   }

   #[test]
   fn test_sibling_directory_with_a_shared_prefix_is_rejected() {
      // A string prefix match, and a different directory. Comparing components
      // rather than characters is what separates them.
      assert!(scope().check("/data/downloads-evil/file.zip").is_err());
      assert!(scope().check("/data/downloadsevil/file.zip").is_err());
   }

   #[test]
   fn test_traversal_above_the_root_cannot_escape() {
      // Each `..` at the root is dropped rather than wrapping around to somewhere
      // the comparison would accept.
      assert!(scope().check("/../../../etc/passwd").is_err());
   }

   #[test]
   fn test_structural_errors_keep_their_own_message() {
      // A malformed path keeps the message every platform already sends for it.
      let message = |p: &str| scope().check(p).unwrap_err().to_string();

      assert_eq!(message(""), "Path Error: path cannot be empty");
      assert_eq!(message("file.txt"), "Path Error: path must be absolute");
      assert_eq!(message("/"), "Path Error: path must have a filename");
   }

   #[test]
   fn test_out_of_scope_message() {
      let error = scope().check("/etc/passwd").unwrap_err();

      assert_eq!(
         error.to_string(),
         "Path Error: path must be inside a download directory"
      );
   }

   #[test]
   fn test_a_root_that_bounds_nothing_is_rejected() {
      // Both fail here rather than at the first download.
      assert!(DownloadScope::new(["downloads"]).is_err());
      assert!(DownloadScope::new([""]).is_err());

      // One bad root among good ones fails too, rather than being dropped and
      // leaving the app bounded by fewer directories than it named.
      assert!(DownloadScope::new(["/data/downloads", "relative"]).is_err());

      let error = DownloadScope::new(["downloads"]).unwrap_err();

      assert_eq!(
         error.to_string(),
         "Path Error: download directory must be absolute"
      );
   }

   #[test]
   fn test_no_roots_at_all_is_rejected() {
      let error = DownloadScope::new(Vec::<PathBuf>::new()).unwrap_err();

      assert_eq!(
         error.to_string(),
         "Path Error: at least one download directory is required"
      );
   }

   #[test]
   fn test_each_root_bounds_its_own_subtree() {
      // The iOS case: two roots the sandbox keeps apart.
      let scope = DownloadScope::new(["/app/Documents/data", "/app/Library/Media"]).unwrap();

      assert!(scope.check("/app/Documents/data/pubs/f.epub").is_ok());
      assert!(scope.check("/app/Library/Media/Videos/f.mp4").is_ok());
      assert!(scope.check("/app/Library/Preferences/f.plist").is_err());
      assert!(scope.check("/app/f.zip").is_err());

      // A `..` cannot hop from one root into anything outside both.
      assert!(
         scope
            .check("/app/Documents/data/../../Library/f.plist")
            .is_err()
      );
   }

   #[test]
   fn test_root_is_normalized_before_it_is_compared() {
      // Both sides are normalized, so a root written with `.` or `..` bounds the
      // same paths as one written plainly.
      let scope = DownloadScope::new(["/data/./media/../downloads"]).unwrap();

      assert!(scope.check("/data/downloads/file.zip").is_ok());
      assert!(scope.check("/data/media/file.zip").is_err());
   }
}
