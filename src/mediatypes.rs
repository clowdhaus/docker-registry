//! Media-types for API objects.

use strum::{Display, EnumProperty, EnumString};

use crate::errors::Result;

// For Docker schema1 types, see https://docs.docker.com/registry/spec/manifest-v2-1/
// For Docker schema2 types, see https://docs.docker.com/registry/spec/manifest-v2-2/
// For OCI types, see https://github.com/opencontainers/image-spec/blob/main/media-types.md

#[derive(EnumProperty, EnumString, Display, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MediaTypes {
  // --- Docker types ---
  /// Manifest, version 2 schema 1.
  #[strum(serialize = "application/vnd.docker.distribution.manifest.v1+json")]
  #[strum(props(Sub = "vnd.docker.distribution.manifest.v1+json"))]
  ManifestV2S1,
  /// Signed manifest, version 2 schema 1.
  #[strum(serialize = "application/vnd.docker.distribution.manifest.v1+prettyjws")]
  #[strum(props(Sub = "vnd.docker.distribution.manifest.v1+prettyjws"))]
  ManifestV2S1Signed,
  /// Manifest, version 2 schema 2.
  #[strum(serialize = "application/vnd.docker.distribution.manifest.v2+json")]
  #[strum(props(Sub = "vnd.docker.distribution.manifest.v2+json"))]
  ManifestV2S2,
  /// Manifest List (aka "fat manifest").
  #[strum(serialize = "application/vnd.docker.distribution.manifest.list.v2+json")]
  #[strum(props(Sub = "vnd.docker.distribution.manifest.list.v2+json"))]
  ManifestList,
  /// Image layer, as a gzip-compressed tar.
  #[strum(serialize = "application/vnd.docker.image.rootfs.diff.tar.gzip")]
  #[strum(props(Sub = "vnd.docker.image.rootfs.diff.tar.gzip"))]
  ImageLayerTgz,
  /// Foreign image layer, as a gzip-compressed tar (e.g. Windows base layers).
  #[strum(serialize = "application/vnd.docker.image.rootfs.foreign.diff.tar.gzip")]
  #[strum(props(Sub = "vnd.docker.image.rootfs.foreign.diff.tar.gzip"))]
  ImageLayerForeignTgz,
  /// Configuration object for a container.
  #[strum(serialize = "application/vnd.docker.container.image.v1+json")]
  #[strum(props(Sub = "vnd.docker.container.image.v1+json"))]
  ContainerConfigV1,

  // --- OCI types ---
  /// OCI Image Manifest.
  #[strum(serialize = "application/vnd.oci.image.manifest.v1+json")]
  #[strum(props(Sub = "vnd.oci.image.manifest.v1+json"))]
  OciImageManifest,
  /// OCI Image Index (multi-platform manifest).
  #[strum(serialize = "application/vnd.oci.image.index.v1+json")]
  #[strum(props(Sub = "vnd.oci.image.index.v1+json"))]
  OciImageIndexV1,
  /// OCI Image Config.
  #[strum(serialize = "application/vnd.oci.image.config.v1+json")]
  #[strum(props(Sub = "vnd.oci.image.config.v1+json"))]
  OciImageConfig,
  /// OCI Image Layer, as an uncompressed tar.
  #[strum(serialize = "application/vnd.oci.image.layer.v1.tar")]
  #[strum(props(Sub = "vnd.oci.image.layer.v1.tar"))]
  OciImageLayerTar,
  /// OCI Image Layer, as a gzip-compressed tar.
  #[strum(serialize = "application/vnd.oci.image.layer.v1.tar+gzip")]
  #[strum(props(Sub = "vnd.oci.image.layer.v1.tar+gzip"))]
  OciImageLayerTgz,
  /// OCI Image Layer, as a zstd-compressed tar.
  #[strum(serialize = "application/vnd.oci.image.layer.v1.tar+zstd")]
  #[strum(props(Sub = "vnd.oci.image.layer.v1.tar+zstd"))]
  OciImageLayerZstd,
  /// OCI Empty descriptor (scratch/unused).
  #[strum(serialize = "application/vnd.oci.empty.v1+json")]
  #[strum(props(Sub = "vnd.oci.empty.v1+json"))]
  OciEmptyV1,

  // --- Generic ---
  /// Generic JSON
  #[strum(serialize = "application/json")]
  #[strum(props(Sub = "json"))]
  ApplicationJson,
}

impl MediaTypes {
  // TODO(lucab): proper error types
  pub fn from_mime(mtype: &mime::Mime) -> Result<Self> {
    match (mtype.type_(), mtype.subtype(), mtype.suffix()) {
      (mime::APPLICATION, mime::JSON, _) => Ok(MediaTypes::ApplicationJson),
      (mime::APPLICATION, subt, None) if subt == "vnd.docker.image.rootfs.diff.tar.gzip" => {
        Ok(MediaTypes::ImageLayerTgz)
      }
      (mime::APPLICATION, subt, None) if subt == "vnd.docker.image.rootfs.foreign.diff.tar.gzip" => {
        Ok(MediaTypes::ImageLayerForeignTgz)
      }
      (mime::APPLICATION, subt, None) if subt == "vnd.oci.image.layer.v1.tar" => Ok(MediaTypes::OciImageLayerTar),
      (mime::APPLICATION, subt, Some(suff)) => match (subt.to_string().as_str(), suff.to_string().as_str()) {
        // Docker
        ("vnd.docker.distribution.manifest.v1", "json") => Ok(MediaTypes::ManifestV2S1),
        ("vnd.docker.distribution.manifest.v1", "prettyjws") => Ok(MediaTypes::ManifestV2S1Signed),
        ("vnd.docker.distribution.manifest.v2", "json") => Ok(MediaTypes::ManifestV2S2),
        ("vnd.docker.distribution.manifest.list.v2", "json") => Ok(MediaTypes::ManifestList),
        ("vnd.docker.image.rootfs.diff.tar.gzip", _) => Ok(MediaTypes::ImageLayerTgz),
        ("vnd.docker.image.rootfs.foreign.diff.tar.gzip", _) => Ok(MediaTypes::ImageLayerForeignTgz),
        ("vnd.docker.container.image.v1", "json") => Ok(MediaTypes::ContainerConfigV1),
        // OCI
        ("vnd.oci.image.manifest.v1", "json") => Ok(MediaTypes::OciImageManifest),
        ("vnd.oci.image.index.v1", "json") => Ok(MediaTypes::OciImageIndexV1),
        ("vnd.oci.image.config.v1", "json") => Ok(MediaTypes::OciImageConfig),
        ("vnd.oci.image.layer.v1.tar", "gzip") => Ok(MediaTypes::OciImageLayerTgz),
        ("vnd.oci.image.layer.v1.tar", "zstd") => Ok(MediaTypes::OciImageLayerZstd),
        ("vnd.oci.empty.v1", "json") => Ok(MediaTypes::OciEmptyV1),
        _ => Err(crate::Error::UnknownMimeType(mtype.clone())),
      },
      _ => Err(crate::Error::UnknownMimeType(mtype.clone())),
    }
  }
  pub fn to_mime(&self) -> mime::Mime {
    match self {
      &MediaTypes::ApplicationJson => Ok(mime::APPLICATION_JSON),
      m => {
        if let Some(s) = m.get_str("Sub") {
          ("application/".to_string() + s).parse()
        } else {
          "application/star".parse()
        }
      }
    }
    .expect("to_mime should be always successful")
  }
}

#[cfg(test)]
mod tests {
  use std::str::FromStr;

  use super::*;

  #[test]
  fn test_roundtrip_to_mime_from_mime() {
    let types = [
      MediaTypes::ManifestV2S1,
      MediaTypes::ManifestV2S1Signed,
      MediaTypes::ManifestV2S2,
      MediaTypes::ManifestList,
      MediaTypes::ImageLayerTgz,
      MediaTypes::ImageLayerForeignTgz,
      MediaTypes::ContainerConfigV1,
      MediaTypes::OciImageManifest,
      MediaTypes::OciImageIndexV1,
      MediaTypes::OciImageConfig,
      MediaTypes::OciImageLayerTar,
      MediaTypes::OciImageLayerTgz,
      MediaTypes::OciImageLayerZstd,
      MediaTypes::OciEmptyV1,
      MediaTypes::ApplicationJson,
    ];
    for mt in &types {
      let mime = mt.to_mime();
      let back = MediaTypes::from_mime(&mime).unwrap();
      assert_eq!(&back, mt, "roundtrip failed for {mt:?}");
    }
  }

  #[test]
  fn test_from_str_roundtrip() {
    let types = [
      "application/vnd.docker.distribution.manifest.v2+json",
      "application/vnd.docker.distribution.manifest.v1+prettyjws",
      "application/vnd.docker.distribution.manifest.list.v2+json",
      "application/vnd.oci.image.manifest.v1+json",
      "application/vnd.oci.image.index.v1+json",
      "application/json",
    ];
    for s in &types {
      let mt = MediaTypes::from_str(s).unwrap();
      assert_eq!(&mt.to_string(), s, "roundtrip failed for {s}");
    }
  }

  #[test]
  fn test_from_mime_unknown_type() {
    let mime: mime::Mime = "text/plain".parse().unwrap();
    let result = MediaTypes::from_mime(&mime);
    assert!(result.is_err());
  }

  #[test]
  fn test_from_mime_unknown_application_subtype() {
    let mime: mime::Mime = "application/vnd.unknown.type.v1+json".parse().unwrap();
    let result = MediaTypes::from_mime(&mime);
    assert!(result.is_err());
  }

  #[test]
  fn test_from_str_invalid() {
    let result = MediaTypes::from_str("not/a/valid/media/type");
    assert!(result.is_err());
  }
}
