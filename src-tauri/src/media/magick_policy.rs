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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_denies_network_and_scripting() {
        for pattern in ["HTTP", "HTTPS", "URL", "FTP", "MVG", "MSL", "TEXT", "LABEL"] {
            assert!(
                POLICY_XML.contains(&format!("rights=\"none\" pattern=\"{pattern}\"")),
                "policy must deny {pattern}"
            );
        }
        assert!(POLICY_XML.contains("domain=\"delegate\" rights=\"none\" pattern=\"*\""));
        // Indirection reads (@file) are blocked.
        assert!(POLICY_XML.contains("domain=\"path\" rights=\"none\" pattern=\"@*\""));
    }

    #[test]
    fn policy_sets_resource_caps() {
        for cap in ["memory", "map", "width", "height", "area", "disk", "file", "thread", "time"] {
            assert!(
                POLICY_XML.contains(&format!("domain=\"resource\" name=\"{cap}\"")),
                "policy must cap {cap}"
            );
        }
    }

    #[test]
    fn ensure_policy_file_writes_contents() {
        let dir = std::env::temp_dir().join(format!("jw-policy-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("tmpdir");

        let path = ensure_policy_file(&dir).expect("write policy");
        assert!(path.is_file());
        let written = fs::read_to_string(&path).expect("read policy");
        assert_eq!(written, POLICY_XML);

        let _ = fs::remove_dir_all(&dir);
    }
}
