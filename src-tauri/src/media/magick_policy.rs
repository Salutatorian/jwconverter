//! Locked-down ImageMagick policy for JW Converter.
//! Deny network/URL and scripting delegates; allow local file raster work only.

use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::AppError;

pub const POLICY_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policymap [
  <!ELEMENT policymap (policy)*>
  <!ATTLIST policymap xmlns CDATA #FIXED ''>
  <!ELEMENT policy EMPTY>
  <!ATTLIST policy xmlns CDATA #FIXED '' domain NMTOKEN #REQUIRED
    name NMTOKEN #IMPLIED pattern CDATA #IMPLIED rights NMTOKEN #IMPLIED
    stealth NMTOKEN #IMPLIED value CDATA #IMPLIED>
]>
<policymap>
  <!-- JW Converter: local file conversion only -->
  <policy domain="delegate" rights="none" pattern="*"/>
  <policy domain="coder" rights="none" pattern="HTTP"/>
  <policy domain="coder" rights="none" pattern="HTTPS"/>
  <policy domain="coder" rights="none" pattern="URL"/>
  <policy domain="coder" rights="none" pattern="FTP"/>
  <policy domain="coder" rights="none" pattern="MVG"/>
  <policy domain="coder" rights="none" pattern="MSL"/>
  <policy domain="coder" rights="none" pattern="TEXT"/>
  <policy domain="coder" rights="none" pattern="LABEL"/>
  <policy domain="path" rights="none" pattern="@*"/>
  <policy domain="resource" name="memory" value="512MiB"/>
  <policy domain="resource" name="map" value="1GiB"/>
  <policy domain="resource" name="width" value="32KP"/>
  <policy domain="resource" name="height" value="32KP"/>
  <policy domain="resource" name="area" value="256MP"/>
  <policy domain="resource" name="disk" value="4GiB"/>
  <policy domain="resource" name="file" value="768"/>
  <policy domain="resource" name="thread" value="4"/>
  <policy domain="resource" name="time" value="300"/>
</policymap>
"#;

/// Write JW Converter's locked policy.xml into the Magick config directory.
pub fn ensure_policy_file(magick_dir: &Path) -> Result<PathBuf, AppError> {
    let path = magick_dir.join("policy.xml");
    fs::write(&path, POLICY_XML).map_err(|error| AppError::DestinationUnavailable {
        detail: format!("Could not write ImageMagick policy.xml: {error}"),
    })?;
    Ok(path)
}
