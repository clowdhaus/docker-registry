//! Render a docker image.

// Docker image format is specified at
// https://github.com/moby/moby/blob/v17.05.0-ce/image/spec/v1.md

use std::{fs, io, path};

use libflate::gzip;

/// Zstd magic number: 0x28B52FFD
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

fn is_zstd(data: &[u8]) -> bool {
  data.len() >= 4 && data[..4] == ZSTD_MAGIC
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
  #[error("wrong target path {}: must be absolute path to existing directory", _0.display())]
  WrongTargetPath(path::PathBuf),
  #[error("io error")]
  Io(#[from] std::io::Error),
}

/// Unpack an ordered list of layers to a target directory.
///
/// Layers must be provided as gzip-compressed tar archives, with lower layers
/// coming first. Target directory must be an existing absolute path.
pub fn unpack(layers: &[Vec<u8>], target_dir: &path::Path) -> Result<(), RenderError> {
  _unpack(layers, target_dir, |mut archive, target_dir| {
    Ok(archive.unpack(target_dir)?)
  })
}

/// Unpack an ordered list of layers to a target directory, filtering
/// file entries by path.
///
/// Layers must be provided as gzip-compressed tar archives, with lower layers
/// coming first. Target directory must be an existing absolute path.
pub fn filter_unpack<P>(layers: &[Vec<u8>], target_dir: &path::Path, predicate: P) -> Result<(), RenderError>
where
  P: Fn(&path::Path) -> bool,
{
  _unpack(layers, target_dir, |mut archive, target_dir| {
    for entry in archive.entries()? {
      let mut entry = entry?;
      let path = entry.path()?;

      if predicate(&path) {
        entry.unpack_in(target_dir)?;
      }
    }

    Ok(())
  })
}

fn decompress(data: &[u8]) -> Result<Box<dyn io::Read + '_>, RenderError> {
  if is_zstd(data) {
    Ok(Box::new(zstd::Decoder::new(data)?))
  } else {
    Ok(Box::new(gzip::Decoder::new(data)?))
  }
}

fn _unpack<U>(layers: &[Vec<u8>], target_dir: &path::Path, unpacker: U) -> Result<(), RenderError>
where
  U: Fn(tar::Archive<Box<dyn io::Read + '_>>, &path::Path) -> Result<(), RenderError>,
{
  if !target_dir.is_absolute() || !target_dir.exists() || !target_dir.is_dir() {
    return Err(RenderError::WrongTargetPath(target_dir.to_path_buf()));
  }
  for l in layers {
    // Unpack layers
    let reader = decompress(l.as_slice())?;
    let mut archive = tar::Archive::new(reader);
    archive.set_preserve_permissions(true);
    archive.set_unpack_xattrs(true);
    unpacker(archive, target_dir)?;

    // Clean whiteouts
    let reader = decompress(l.as_slice())?;
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries()? {
      let file = entry?;
      let path = file.path()?;
      let parent = path.parent().unwrap_or_else(|| path::Path::new("/"));
      if let Some(fname) = path.file_name() {
        let wh_name = fname.to_string_lossy();
        if wh_name == ".wh..wh..opq" {
          //TODO(lucab): opaque whiteout, dir removal
        } else if wh_name.starts_with(".wh.") {
          let rel_parent = path::PathBuf::from("./".to_string() + &parent.to_string_lossy());

          // Remove real file behind whiteout
          let real_name = wh_name.trim_start_matches(".wh.");
          let abs_real_path = target_dir.join(&rel_parent).join(real_name);
          remove_whiteout(abs_real_path)?;

          // Remove whiteout place-holder
          let abs_wh_path = target_dir.join(&rel_parent).join(fname);
          remove_whiteout(abs_wh_path)?;
        };
      }
    }
  }
  Ok(())
}

// Whiteout files in archive may not exist on filesystem if they were
// filtered out via filter_unpack.  If not found, that's ok and the
// error is non-fatal.  Otherwise still return error for other
// failures.
fn remove_whiteout(path: path::PathBuf) -> io::Result<()> {
  if path.is_dir() {
    let res = fs::remove_dir_all(&path);
    match res {
      Ok(_) => Ok(()),
      Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
      Err(e) => Err(e),
    }
  } else {
    let res = fs::remove_file(&path);
    match res {
      Ok(_) => Ok(()),
      Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
      Err(e) => Err(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{io::Write, path::Path};

  use super::*;

  /// Helper: create a gzip-compressed tar archive containing a single file.
  fn make_layer(file_name: &str, content: &[u8]) -> Vec<u8> {
    let mut tar_buf = Vec::new();
    {
      let mut builder = tar::Builder::new(&mut tar_buf);
      let mut header = tar::Header::new_gnu();
      header.set_path(file_name).unwrap();
      header.set_size(content.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append(&header, content).unwrap();
      builder.finish().unwrap();
    }
    let mut gz_buf = Vec::new();
    {
      let mut encoder = gzip::Encoder::new(&mut gz_buf).unwrap();
      encoder.write_all(&tar_buf).unwrap();
      encoder.finish().into_result().unwrap();
    }
    gz_buf
  }

  /// Helper: create a layer with a whiteout marker file.
  fn make_whiteout_layer(whiteout_path: &str) -> Vec<u8> {
    make_layer(whiteout_path, b"")
  }

  #[test]
  fn test_unpack_single_layer() {
    let dir = tempfile::tempdir().unwrap();
    let layer = make_layer("hello.txt", b"hello world");
    unpack(&[layer], dir.path()).unwrap();
    let content = fs::read_to_string(dir.path().join("hello.txt")).unwrap();
    assert_eq!(content, "hello world");
  }

  #[test]
  fn test_unpack_multiple_layers() {
    let dir = tempfile::tempdir().unwrap();
    let layer1 = make_layer("file1.txt", b"content1");
    let layer2 = make_layer("file2.txt", b"content2");
    unpack(&[layer1, layer2], dir.path()).unwrap();
    assert_eq!(fs::read_to_string(dir.path().join("file1.txt")).unwrap(), "content1");
    assert_eq!(fs::read_to_string(dir.path().join("file2.txt")).unwrap(), "content2");
  }

  #[test]
  fn test_unpack_layer_overwrites_previous() {
    let dir = tempfile::tempdir().unwrap();
    let layer1 = make_layer("file.txt", b"old");
    let layer2 = make_layer("file.txt", b"new");
    unpack(&[layer1, layer2], dir.path()).unwrap();
    assert_eq!(fs::read_to_string(dir.path().join("file.txt")).unwrap(), "new");
  }

  #[test]
  fn test_unpack_relative_path_rejected() {
    let layer = make_layer("hello.txt", b"hello");
    let result = unpack(&[layer], Path::new("relative/path"));
    assert!(result.is_err());
  }

  #[test]
  fn test_unpack_nonexistent_path_rejected() {
    let layer = make_layer("hello.txt", b"hello");
    let result = unpack(&[layer], Path::new("/nonexistent/path/that/does/not/exist"));
    assert!(result.is_err());
  }

  #[test]
  fn test_unpack_empty_layers() {
    let dir = tempfile::tempdir().unwrap();
    unpack(&[], dir.path()).unwrap();
    // Should succeed with no files created
  }

  #[test]
  fn test_filter_unpack_includes_matching() {
    let dir = tempfile::tempdir().unwrap();
    let layer = make_layer("include-me.txt", b"included");
    filter_unpack(&[layer], dir.path(), |p| p.to_string_lossy().contains("include")).unwrap();
    assert!(dir.path().join("include-me.txt").exists());
  }

  #[test]
  fn test_filter_unpack_excludes_non_matching() {
    let dir = tempfile::tempdir().unwrap();
    let layer = make_layer("exclude-me.txt", b"excluded");
    filter_unpack(&[layer], dir.path(), |p| p.to_string_lossy().contains("include")).unwrap();
    assert!(!dir.path().join("exclude-me.txt").exists());
  }

  #[test]
  fn test_whiteout_removes_file() {
    let dir = tempfile::tempdir().unwrap();
    let layer1 = make_layer("myfile.txt", b"content");
    let layer2 = make_whiteout_layer(".wh.myfile.txt");
    unpack(&[layer1, layer2], dir.path()).unwrap();
    assert!(!dir.path().join("myfile.txt").exists());
  }

  #[test]
  fn test_unpack_invalid_gzip() {
    let dir = tempfile::tempdir().unwrap();
    let result = unpack(&[b"not gzip data".to_vec()], dir.path());
    assert!(result.is_err());
  }

  /// Helper: create a zstd-compressed tar archive containing a single file.
  fn make_zstd_layer(file_name: &str, content: &[u8]) -> Vec<u8> {
    let mut tar_buf = Vec::new();
    {
      let mut builder = tar::Builder::new(&mut tar_buf);
      let mut header = tar::Header::new_gnu();
      header.set_path(file_name).unwrap();
      header.set_size(content.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append(&header, content).unwrap();
      builder.finish().unwrap();
    }
    zstd::encode_all(tar_buf.as_slice(), 3).unwrap()
  }

  #[test]
  fn test_unpack_zstd_single_layer() {
    let dir = tempfile::tempdir().unwrap();
    let layer = make_zstd_layer("hello.txt", b"hello zstd");
    unpack(&[layer], dir.path()).unwrap();
    let content = fs::read_to_string(dir.path().join("hello.txt")).unwrap();
    assert_eq!(content, "hello zstd");
  }

  #[test]
  fn test_unpack_zstd_multiple_layers() {
    let dir = tempfile::tempdir().unwrap();
    let layer1 = make_zstd_layer("file1.txt", b"content1");
    let layer2 = make_zstd_layer("file2.txt", b"content2");
    unpack(&[layer1, layer2], dir.path()).unwrap();
    assert_eq!(fs::read_to_string(dir.path().join("file1.txt")).unwrap(), "content1");
    assert_eq!(fs::read_to_string(dir.path().join("file2.txt")).unwrap(), "content2");
  }

  #[test]
  fn test_unpack_mixed_gzip_and_zstd() {
    let dir = tempfile::tempdir().unwrap();
    let gz_layer = make_layer("from_gzip.txt", b"gzip content");
    let zstd_layer = make_zstd_layer("from_zstd.txt", b"zstd content");
    unpack(&[gz_layer, zstd_layer], dir.path()).unwrap();
    assert_eq!(
      fs::read_to_string(dir.path().join("from_gzip.txt")).unwrap(),
      "gzip content"
    );
    assert_eq!(
      fs::read_to_string(dir.path().join("from_zstd.txt")).unwrap(),
      "zstd content"
    );
  }

  #[test]
  fn test_filter_unpack_zstd() {
    let dir = tempfile::tempdir().unwrap();
    let layer = make_zstd_layer("include-me.txt", b"included");
    filter_unpack(&[layer], dir.path(), |p| p.to_string_lossy().contains("include")).unwrap();
    assert!(dir.path().join("include-me.txt").exists());
  }

  #[test]
  fn test_whiteout_removes_file_zstd() {
    let dir = tempfile::tempdir().unwrap();
    let layer1 = make_zstd_layer("myfile.txt", b"content");
    let layer2 = make_zstd_layer(".wh.myfile.txt", b"");
    unpack(&[layer1, layer2], dir.path()).unwrap();
    assert!(!dir.path().join("myfile.txt").exists());
  }

  #[test]
  fn test_is_zstd_detection() {
    assert!(is_zstd(&[0x28, 0xB5, 0x2F, 0xFD, 0x00]));
    assert!(!is_zstd(&[0x1F, 0x8B, 0x08, 0x00])); // gzip
    assert!(!is_zstd(&[0x00, 0x01, 0x02])); // too short
    assert!(!is_zstd(&[]));
  }
}
